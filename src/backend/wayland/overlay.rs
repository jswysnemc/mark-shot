// ── Wayland Screen Overlay Implementation ─────────────────────────────────────
//
// this module implements the 'ScreenOverlay' trait for wayland using SCTK
// It is responsible for creating surfaces, mapping them to the correct outputs
// via xdg windows or layer_shell, managing shm pixel buffers, and driving the event loop
//
// by default, auto decides what to use, for everything a fullscreen window
// for scrolling wm (for now only niri) layer, since creating a window there will start a clunky animation for a screenshot app
// and for COSMIC, since force fullscreen window is unstable, at least for now (focus grabbing + gray cosmic ui)
use std::io::ErrorKind;
use std::time::{Duration, Instant};
use wayland_client::backend::WaylandError;

mod buffer_budget;
mod frame_mode;
pub(crate) mod frame_pixels;
pub(super) mod frame_staging;
mod frozen_background;
mod image_window;
mod poll_sources;
mod presentation;
mod scroll;
pub(crate) mod scroll_layer;
mod scroll_window;
pub mod state;
mod visibility;

use crate::backend::ScreenOverlay;
use crate::backend::wayland::utils::surface::InputRegion;
use crate::types::{CursorIcon, DamageRect, Output, OverlayEvent};
use log::info;
use smithay_client_toolkit::compositor::{FrameCallbackData, Region};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{KeyboardInteractivity, Layer};
use smithay_client_toolkit::shm::slot::SlotPool;
use state::Probe;

const CONFIGURE_TIMEOUT: Duration = Duration::from_secs(5);
const PROBE_TIMEOUT: Duration = Duration::from_millis(300);

pub struct WaylandOverlay {
    runtime: state::OverlayRunTime,
    layer: bool,
    faulted: usize,
    composited_background: bool,
    targets: Vec<wayland_client::protocol::wl_output::WlOutput>,
    suspended: bool,
    scrolling: Option<usize>,
    scroll_window: Option<scroll_window::ScrollWindow>,
    scroll_panel_rect: Option<tiny_skia::Rect>,
    task_source: Option<crate::backend::task_wake::Source>,
    theme_source: Option<crate::theme::config_watch::Watch>,
    controller: Option<std::sync::Arc<crate::features::single_instance::Controller>>,
}

impl WaylandOverlay {
    pub fn new(
        connection: wayland_client::Connection,
        layer: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let rt = state::OverlayRunTime::new(&connection)?;
        let has_layer_shell = rt.state.layer_shell.is_some();
        if layer && !has_layer_shell {
            info!("compositor has no wlr-layer-shell, the overlay opens as a window");
        }

        Ok(Self {
            runtime: rt,
            layer: layer && has_layer_shell,
            faulted: 0,
            composited_background: false,
            targets: Vec::new(),
            suspended: false,
            scrolling: None,
            scroll_window: None,
            scroll_panel_rect: None,
            task_source: None,
            theme_source: super::theme_notifications::source(),
            controller: crate::features::single_instance::context::current(),
        })
    }
}

impl ScreenOverlay for WaylandOverlay {
    /// 【图片编辑】【窗口启用】按 size 设置初始逻辑尺寸并使用普通窗口，返回是否成功
    fn configure_image_window(&mut self, size: (u32, u32)) -> bool {
        self.layer = false;
        self.runtime.state.image_window = Some(size);
        true
    }
    /// 【图片编辑】【窗口尺寸】返回最后一次合成器配置的尺寸，无额外参数
    fn image_window_size(&self) -> Option<(u32, u32)> {
        self.runtime.state.image_window
    }
    /// 【截图覆盖层】【原生背景能力】无参数，单表面模式允许在原生缓冲中合成同网格冻结背景
    fn supports_native_frame_background(&self) -> bool {
        self.composited_background
    }
    /// 【截图覆盖层】【前景消费能力】无参数，完整合成模式支持按批消费CPU前景，其他模式保留既有接口
    fn supports_consumed_frame_background(&self) -> bool {
        self.composited_background
    }
    /// 【截图覆盖层】【消费前景提交】按monitor_idx、pixels、background和damage交付原始网格并归还已读取CPU字节，返回协议结果
    fn stage_consumed_frame_with_background(
        &mut self,
        monitor_idx: usize,
        pixels: &mut tiny_skia::Pixmap,
        background: &crate::image_source::ImageSource,
        damage: Option<DamageRect>,
    ) -> crate::AppResult {
        self.stage_with_pixels(
            monitor_idx,
            frame_pixels::FramePixels::Consumed(pixels),
            Some(background),
            damage,
        )
    }

    /// 【截图覆盖层】【原生背景提交】按monitor_idx、pixels、background和damage提交冻结背景与原始前景，返回协议结果
    fn stage_frame_with_background(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
        background: &crate::image_source::ImageSource,
        damage: Option<DamageRect>,
    ) -> crate::AppResult {
        self.stage_pixels(monitor_idx, pixels, Some(background), damage)
    }
    /// 【截图覆盖层】【背景模式】按enabled启用完整帧，返回实际模式，不额外分配整屏共享背景
    fn set_composited_background(&mut self, enabled: bool) -> bool {
        self.select_frame_mode(enabled)
    }
    /// 【滚动截图】【输入角色】按 monitor_idx 切换捕获表面并释放其他输出，返回协议结果
    fn begin_scroll(&mut self, monitor_idx: usize) -> crate::AppResult {
        self.start_scroll_surface(monitor_idx)
    }
    /// 【滚动截图】【普通面板】按 rect 和 paused 保留普通窗口布局，不改变支持 Layer Shell 的呈现方式
    fn configure_scroll_panel(&mut self, rect: tiny_skia::Rect, paused: bool) -> crate::AppResult {
        self.scroll_panel_rect = Some(rect);
        self.configure_scroll_window(rect, paused);
        self.resume_scroll_controls(paused)
    }
    /// 【滚动截图】【当前角色】返回是否正在使用独立普通控制窗口，无参数
    fn is_scroll_window(&self) -> bool {
        self.scrolling.is_some() && self.scroll_window.is_some()
    }
    /// 【滚动截图】【实际边界】按 monitor_idx 返回合成器缩小后的控制区域，浮层或无约束时返回空值
    fn scroll_panel_limits(&self, monitor_idx: usize) -> Option<(u32, u32)> {
        let surface = self.runtime.state.surfaces.get(&monitor_idx)?;
        match &surface.role {
            crate::backend::wayland::utils::surface::Role::ScrollPopup(popup) => popup.limits,
            _ => None,
        }
    }
    /// 【滚动截图】【污染检查】以 frame 和 region 检查实际控制位置，返回是否接受当前帧，不依赖窗口位置猜测
    fn allow_scroll_frame(
        &mut self,
        frame: &crate::types::MonitorFrame,
        region: tiny_skia::Rect,
    ) -> crate::AppResult<bool> {
        self.allow_window_scroll_frame(frame, region)
    }
    /// 【滚动截图】【隐藏状态】返回普通控制窗口是否处于捕获避让状态，无参数
    fn scroll_controls_hidden(&self) -> bool {
        self.scroll_window
            .as_ref()
            .is_some_and(|window| window.hidden)
    }
    /// 【滚动截图】【输出绑定】按表面索引查找原输出对象，返回当前仍存活的输出
    fn surface_output(&self, monitor_idx: usize) -> Option<&Output> {
        let target = self.targets.get(monitor_idx)?;
        self.runtime
            .state
            .outputs
            .iter()
            .find(|output| output.wl_output.as_ref() == Some(target))
    }
    /// 【主选择区】【协议启用】在运行时 globals 与当前座席上绑定协议，无参数，返回支持状态
    fn enable_primary_selection(&mut self) -> bool {
        let rt = &mut self.runtime;
        let supported = rt.state.primary_selection.enable(
            &rt.globals,
            rt.state.seat.seats(),
            &rt.event_queue.handle(),
        );
        rt.state.pending_flush = true;
        supported
    }
    /// 【主选择区】【来源拥有】按 text 创建当前座席来源，返回是否已设置，刷新由本次事件循环完成
    fn set_primary_text(&mut self, text: &str) -> bool {
        let rt = &mut self.runtime;
        let changed = rt
            .state
            .primary_selection
            .set(text, &rt.event_queue.handle());
        rt.state.pending_flush |= changed;
        changed
    }
    /// 【主选择区】【接收请求】用 id 与 limit 请求当前来源，立即可用时仍经事件队列交付，无返回值
    fn request_primary_text(&mut self, id: u64, limit: usize) {
        let state = &mut self.runtime.state;
        if let Some(text) = state.primary_selection.request(id, limit) {
            state
                .events
                .push_back(OverlayEvent::PrimaryText { id, text });
        }
        state.pending_flush = true;
    }
    /// 【主选择区】【接收取消】取消 id 对应来源管道，无返回值，不撤销当前复制来源
    fn cancel_primary_text(&mut self, id: u64) {
        self.runtime.state.primary_selection.cancel(id);
    }
    /// 【截图覆盖层】【任务通知】按需创建独立通知源，返回完成句柄；普通截图启动不分配该管道
    fn task_wake(&mut self) -> crate::AppResult<crate::backend::task_wake::Wake> {
        if self.task_source.is_none() {
            self.task_source = Some(crate::backend::task_wake::Source::new()?);
        }
        Ok(self.task_source.as_ref().unwrap().wake())
    }
    /// 【输入法】【原生同步】同步 snapshot 后刷新连接，空值停用正文输入；不依赖画布重绘，返回协议结果
    fn set_text_input(
        &mut self,
        snapshot: Option<&crate::types::text_input::Snapshot>,
    ) -> crate::AppResult {
        self.runtime.state.text_input.sync(snapshot);
        self.runtime.event_queue.flush()?;
        Ok(())
    }
    /// 【截图弹层】【原生同步】将 frame 交给真实 Wayland 弹出表面；空值释放菜单抓取，返回原生支持状态
    fn set_popup(
        &mut self,
        frame: Option<crate::types::popup::PopupFrame<'_>>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match frame {
            Some(frame) => self
                .runtime
                .state
                .sync_popup(frame, &self.runtime.event_queue.handle())?,
            None if self.runtime.state.popup.is_none() => return Ok(true),
            None => self.runtime.state.popup = None,
        }
        self.runtime.event_queue.flush()?;
        Ok(true)
    }
    /// 【截图覆盖层】【暂停显示】销毁当前表面以露出真实桌面，保留目标输出；返回暂停结果
    fn suspend(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.suspend_surfaces()
    }

    /// 【截图覆盖层】【恢复显示】按保留的输出身份恢复表面，避免依赖可能变化的全局索引；返回恢复结果
    fn resume(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.resume_surfaces()
    }
    /// Preloads memory for all monitors
    /// this is done in advance so as not to waste extra time on it later.
    ///
    /// function is called before capturing
    /// At this point, we can't know which monitor is active, so it's called for literally all of them.
    ///
    /// TODO: on -m, instead of preloading memory for all amonitors, preloading max possible size of one monitor
    fn prefault(&mut self) {
        if self.composited_background || self.runtime.state.subcompositor.is_some() {
            return;
        }
        let state = &mut self.runtime.state;
        let hidpi = state.viewporter.is_some();
        let size = buffer_budget::size(state.outputs.iter(), hidpi, true, None);
        fault_in(&mut state.pool, &mut self.faulted, size);
    }

    /// 【截图覆盖层】【目标呈现】按 targets 创建原输出角色与缓冲，返回配置结果
    fn present(&mut self, targets: &[usize]) -> Result<(), Box<dyn std::error::Error>> {
        self.present_surfaces(targets)
    }

    fn stage_frame(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
        damage: Option<DamageRect>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.stage_scroll_window(monitor_idx, pixels)? {
            return Ok(());
        }
        self.stage_pixels(monitor_idx, pixels, None, damage)
    }

    /// 【原生帧】【不透明交付】按 monitor_idx、pixels 和 damage 直接转换已合成场景，跳过透明页检测和背景重读，返回协议结果
    fn stage_opaque_frame(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
        damage: Option<DamageRect>,
    ) -> crate::AppResult {
        self.stage_with_pixels(
            monitor_idx,
            frame_pixels::FramePixels::Opaque(pixels),
            None,
            damage,
        )
    }

    /// 【滚动截图】【局部提交】按monitor_idx和frame交付面板，浮层边线使用独立子表面，返回协议结果
    fn stage_scroll_panel(
        &mut self,
        monitor_idx: usize,
        frame: &crate::types::scroll_panel::ScrollPanelFrame<'_>,
    ) -> crate::AppResult {
        if self.stage_local_scroll_window(monitor_idx, frame)? {
            return Ok(());
        }
        let runtime = &mut self.runtime;
        let state = &mut runtime.state;
        let surface = state
            .surfaces
            .get_mut(&monitor_idx)
            .ok_or("the scroll surface is gone")?;
        let layer = surface
            .scroll_layer
            .as_mut()
            .ok_or("the scroll layer is not initialized")?;
        layer.paint(
            &surface.surface,
            &state.compositor_state,
            state
                .subcompositor
                .as_ref()
                .ok_or("the compositor has no subsurfaces")?,
            state.viewporter.as_ref(),
            &mut state.pool,
            &runtime.event_queue.handle(),
            frame,
        )?;
        surface.surface.frame(
            &runtime.event_queue.handle(),
            FrameCallbackData(surface.surface.clone()),
        );
        surface.frame_sent = Some(Instant::now());
        surface.surface.damage_buffer(0, 0, 1, 1);
        surface.surface.commit();
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.runtime.event_queue.flush()?;
        Ok(())
    }

    fn next_event(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<OverlayEvent, Box<dyn std::error::Error>> {
        let rt = &mut self.runtime;

        loop {
            // prepare the wayland connection socket for reading incoming server events
            if let Some(guard) = rt.event_queue.prepare_read() {
                match guard.read() {
                    Ok(_) => {}
                    Err(WaylandError::Io(e)) if e.kind() == ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e.into()),
                }
            }
            rt.event_queue.dispatch_pending(&mut rt.state)?;
            frame_staging::released(&mut rt.state, !self.composited_background);
            rt.state.primary_selection.progress(&mut rt.state.events);

            if rt.state.pending_flush {
                rt.state.pending_flush = false;
                rt.event_queue.flush()?;
            }

            if let Some(mut ev) = rt.state.events.pop_front() {
                if matches!(
                    ev,
                    OverlayEvent::PointerButton {
                        timestamp: Some(_),
                        ..
                    }
                ) {
                    rt.state.primary_selection.advance();
                }
                if let OverlayEvent::TextInput(update) = &mut ev {
                    rt.state.text_input.validate(update);
                }
                // 1. 【截图弹层】【序列号保留】只清理已经分派的旧按下记录，未来排队输入不能挤掉当前菜单的抓取身份
                if let OverlayEvent::PointerButton {
                    serial: Some(serial),
                    ..
                }
                | OverlayEvent::PopupSerial(serial) = &ev
                {
                    crate::backend::wayland::overlay_popup::retain_current_press(
                        &mut rt.state.popup_grabs,
                        *serial,
                    );
                }
                // 2. 【截图输入】【移动合并】保留范围供点击判断使用，界面仍只处理最终坐标
                if let OverlayEvent::PointerMove { .. } = ev {
                    return Ok(state::pointer_coalescing::coalesce(
                        ev,
                        &mut rt.state.events,
                    ));
                }
                if let OverlayEvent::FrameDone { .. } = ev
                    && !rt.state.events.is_empty()
                {
                    continue;
                }
                return Ok(ev);
            }

            // 3. 【截图输入】【统一等待】输入和控制连接同时等待，服务请求不会新增绘制或轮询线程
            match poll_sources::wait(
                rt,
                self.task_source.as_ref(),
                self.theme_source.as_ref(),
                self.controller.as_ref(),
                timeout,
            )? {
                poll_sources::Wake::Tick => return Ok(OverlayEvent::Tick),
                poll_sources::Wake::Wayland | poll_sources::Wake::Progress => {}
            }
        }
    }

    fn discovered_outputs(&self) -> &[Output] {
        &self.runtime.state.outputs
    }

    fn hidpi(&self) -> bool {
        self.runtime.state.viewporter.is_some()
    }

    fn ready(&self, monitor_idx: usize) -> bool {
        if self.runtime.state.surfaces.get(&monitor_idx).is_some_and(|surface| {
            matches!(&surface.role, crate::backend::wayland::utils::surface::Role::ScrollPopup(popup) if popup.pending)
        }) {
            return false;
        }
        if self
            .runtime
            .state
            .surfaces
            .get(&monitor_idx)
            .is_some_and(|surface| !frame_staging::has_writable_buffer(surface))
        {
            return false;
        }
        self.runtime
            .state
            .surfaces
            .get(&monitor_idx)
            .and_then(|sd| sd.frame_sent)
            .is_none_or(|sent| sent.elapsed() >= crate::backend::FRAME_FALLBACK)
    }

    fn active_output(&mut self) -> Option<usize> {
        let rt = &mut self.runtime;
        let qh = rt.event_queue.handle();
        let shell = rt.state.layer_shell.as_ref()?;

        let surface = rt.state.compositor_state.create_surface(&qh);
        let layer =
            shell.create_layer_surface(&qh, surface, Layer::Overlay, Some("lumine-probe"), None);
        layer.set_size(1, 1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        if let Ok(region) = Region::new(&rt.state.compositor_state) {
            layer
                .wl_surface()
                .set_input_region(Some(region.wl_region()));
        }
        layer.commit();
        rt.state.probe = Some(Probe {
            layer,
            buffer: None,
            output: None,
        });

        let deadline = Instant::now() + PROBE_TIMEOUT;
        while rt.state.probe.as_ref().is_some_and(|p| p.output.is_none())
            && Instant::now() < deadline
        {
            if rt.event_queue.roundtrip(&mut rt.state).is_err() {
                break;
            }
            if rt.state.probe.as_ref().is_some_and(|p| p.output.is_none()) {
                std::thread::sleep(Duration::from_millis(2));
            }
        }

        let output = rt.state.probe.take()?.output?;
        let _ = rt.event_queue.flush();
        rt.state
            .outputs
            .iter()
            .position(|o| o.wl_output.as_ref() == Some(&output))
    }

    fn retain_outputs(&mut self, keep: &[usize]) -> Result<(), Box<dyn std::error::Error>> {
        self.targets = keep
            .iter()
            .filter_map(|&index| self.targets.get(index).cloned())
            .collect();
        let rt = &mut self.runtime;
        let remap = |idx: usize| keep.iter().position(|&k| k == idx);

        for (idx, sd) in std::mem::take(&mut rt.state.surfaces) {
            if let Some(new_idx) = remap(idx) {
                rt.state.surfaces.insert(new_idx, sd);
            }
        }

        rt.state.pointer_surface_idx = rt.state.pointer_surface_idx.and_then(remap);
        rt.state.events.retain_mut(|ev| {
            if let OverlayEvent::PointerMove { history, .. } = ev {
                history.retain_mut(|range| {
                    if let Some(index) = remap(range.monitor_idx) {
                        range.monitor_idx = index;
                        true
                    } else {
                        false
                    }
                });
            }
            match ev {
                OverlayEvent::PointerMove { monitor_idx, .. }
                | OverlayEvent::Focus { monitor_idx }
                | OverlayEvent::FocusLost { monitor_idx }
                | OverlayEvent::FrameDone { monitor_idx }
                | OverlayEvent::ScrollPanelConfigured { monitor_idx } => {
                    match remap(*monitor_idx) {
                        Some(idx) => {
                            *monitor_idx = idx;
                            true
                        }
                        None => false,
                    }
                }
                _ => true,
            }
        });

        rt.event_queue.flush()?;
        Ok(())
    }

    fn set_background(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.upload_background_pixels(monitor_idx, pixels, width, height)
    }

    /// 【冻结背景】【来源提交】按monitor_idx和source上传只读背景，返回协议结果，不读取完整原件到另一个CPU帧
    fn set_background_source(
        &mut self,
        monitor_idx: usize,
        source: &crate::image_source::ImageSource,
    ) -> crate::AppResult {
        self.upload_background_source(monitor_idx, source)
    }

    fn set_cursor(&mut self, icon: CursorIcon) {
        self.runtime.state.apply_cursor(icon);
    }

    fn clear_background(&mut self, monitor_idx: usize) -> Result<(), Box<dyn std::error::Error>> {
        let rt = &mut self.runtime;
        let Some(sd) = rt.state.surfaces.get_mut(&monitor_idx) else {
            return Err("surface not found".into());
        };
        // detach buffer from background subsurface so live content reveals
        if let Some(background) = sd.background.take() {
            background.surface.attach(None, 0, 0);
            background.surface.commit();
        }
        rt.event_queue.flush()?;
        Ok(())
    }

    fn set_input_rect(&mut self, monitor_idx: usize, rect: Option<(i32, i32, i32, i32)>) {
        let rt = &mut self.runtime;
        let Some(sd) = rt.state.surfaces.get_mut(&monitor_idx) else {
            return;
        };
        // empty input region lets clicks and wheel pass through to the window below
        sd.input = match rect {
            Some(_) if sd.canvas_rect.is_some() => InputRegion::Full,
            Some((x, y, w, h)) => InputRegion::Rect(x, y, w, h),
            None => InputRegion::Nothing,
        };

        // unmapped surfaces cannot commit state; defer until configure
        if !sd.mapped {
            return;
        }
        sd.apply_input(&rt.state.compositor_state);
        sd.surface.commit();
        let _ = rt.event_queue.flush();
    }
}

fn fault_in(pool: &mut SlotPool, faulted: &mut usize, size: usize) {
    if size <= *faulted {
        return;
    }
    if let Ok(slot) = pool.new_slot(size) {
        pool.raw_data_mut(&slot).fill(0);
        *faulted = size;
    }
}
