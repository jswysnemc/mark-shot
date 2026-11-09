//! 【X11覆盖层】【呈现契约】复用共享RGBA合成路径并管理覆盖窗口生命周期

use super::*;
use crate::{
    backend::ScreenOverlay,
    types::{CursorIcon, DamageRect},
};
use std::time::Duration;
use x11rb::{
    connection::Connection,
    protocol::{
        shape::{ConnectionExt as _, SK, SO},
        xproto::*,
    },
};

impl ScreenOverlay for Overlay {
    /// 【X11覆盖层】【输入法同步】按snapshot接入当前正文输入法，空值关闭上下文，返回协议结果
    fn set_text_input(
        &mut self,
        snapshot: Option<&crate::types::text_input::Snapshot>,
    ) -> crate::AppResult {
        self.sync_text_input(snapshot)
    }
    /// 【X11覆盖层】【原生菜单】按frame同步菜单或关闭，返回协议支持状态
    fn set_popup(
        &mut self,
        frame: Option<crate::types::popup::PopupFrame<'_>>,
    ) -> crate::AppResult<bool> {
        self.popup_frame(frame)
    }
    /// 【X11主选择区】【有界读取】按id和limit读取PRIMARY并投递对应正文事件，无返回值
    fn request_primary_text(&mut self, id: u64, limit: usize) {
        let text = crate::backend::x11::clipboard::read("UTF8_STRING", true, limit)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok());
        self.pending
            .push_back(OverlayEvent::PrimaryText { id, text });
    }
    /// 【X11主选择区】【过期取消】按id删除尚未交付的读取事件，无返回值
    fn cancel_primary_text(&mut self, id: u64) {
        self.pending.retain(|event| !matches!(event, OverlayEvent::PrimaryText { id: current, .. } if *current == id));
    }
    /// 【X11滚动截图】【开始会话】按monitor_idx切换局部控制窗口，返回协议结果
    fn begin_scroll(&mut self, monitor_idx: usize) -> crate::AppResult {
        self.start_scroll(monitor_idx)
    }
    /// 【X11滚动截图】【面板配置】按rect和paused更新布局并在暂停时恢复控件，返回成功
    fn configure_scroll_panel(&mut self, rect: tiny_skia::Rect, paused: bool) -> crate::AppResult {
        self.panel_rect = Some(rect);
        if paused {
            self.panel_hidden = false;
        }
        Ok(())
    }
    /// 【X11滚动截图】【面板角色】无参数，返回是否已启用局部控制窗口
    fn is_scroll_window(&self) -> bool {
        self.scrolling.is_some()
    }
    /// 【X11滚动截图】【可见状态】无参数，返回是否因捕获重叠隐藏面板
    fn scroll_controls_hidden(&self) -> bool {
        self.panel_hidden
    }
    /// 【X11滚动截图】【帧隔离】按frame与region检查控制窗口重叠，返回可接受状态
    fn allow_scroll_frame(
        &mut self,
        _frame: &crate::types::MonitorFrame,
        region: tiny_skia::Rect,
    ) -> crate::AppResult<bool> {
        self.allow_scroll(region)
    }
    /// 【X11滚动截图】【局部交付】按monitor_idx与frame提交局部面板，返回协议结果
    fn stage_scroll_panel(
        &mut self,
        monitor_idx: usize,
        frame: &crate::types::scroll_panel::ScrollPanelFrame<'_>,
    ) -> crate::AppResult {
        self.scroll_frame(monitor_idx, frame)
    }
    /// 【X11覆盖层】【图片窗口】按size选择普通窗口角色，返回支持状态
    fn configure_image_window(&mut self, size: (u32, u32)) -> bool {
        self.image_size = Some(size);
        true
    }
    /// 【X11覆盖层】【窗口尺寸】无参数，返回当前图片窗口尺寸
    fn image_window_size(&self) -> Option<(u32, u32)> {
        self.image_size
    }
    /// 【X11覆盖层】【背景模式】按enabled接收模式请求；X11始终需要调用方完整合成，返回真实启用状态
    fn set_composited_background(&mut self, _enabled: bool) -> bool {
        true
    }
    /// 【X11覆盖层】【预先准备】无参数，资源在首次呈现时分配，无返回值
    fn prefault(&mut self) {}
    /// 【X11覆盖层】【后台通知】无参数，返回当前窗口循环的任务通知句柄
    fn task_wake(&mut self) -> crate::AppResult<task_wake::Wake> {
        Ok(self.wake.wake())
    }
    /// 【X11覆盖层】【窗口创建】按targets创建隐藏窗口，返回错误，输出索引保持业务选择顺序
    fn present(&mut self, targets: &[usize]) -> crate::AppResult {
        self.popup = None;
        self.popup_grab = None;
        self.text_focus(None)?;
        self.surfaces.clear();
        for &target in targets {
            let output = self
                .outputs
                .get(target)
                .ok_or("invalid X11 display target")?;
            let dimensions = output.info.logical_size.ok_or("X11 output size missing")?;
            let size = self
                .image_size
                .unwrap_or((dimensions.0 as u32, dimensions.1 as u32));
            self.surfaces.push((
                target,
                Surface::new(
                    self.display.clone(),
                    output.info.location,
                    size,
                    self.image_size.is_none(),
                )?,
            ));
        }
        Ok(())
    }
    /// 【X11覆盖层】【完整帧提交】按monitor_idx、pixels和damage上传已合成像素，返回协议错误
    fn stage_frame(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
        _damage: Option<DamageRect>,
    ) -> crate::AppResult {
        let surface = &mut self
            .surfaces
            .get_mut(monitor_idx)
            .ok_or("X11 overlay surface missing")?
            .1;
        let first = !surface.mapped;
        surface.present(pixels)?;
        if first {
            self.display
                .connection
                .set_input_focus(
                    InputFocus::POINTER_ROOT,
                    surface.window,
                    x11rb::CURRENT_TIME,
                )?
                .check()?;
        }
        Ok(())
    }
    /// 【X11覆盖层】【批量交付】无参数，发送已排队请求，返回连接错误
    fn flush(&mut self) -> crate::AppResult {
        self.display.connection.flush()?;
        Ok(())
    }
    /// 【X11覆盖层】【事件读取】按timeout等待输入或后台完成，返回共享事件
    fn next_event(&mut self, timeout: Option<Duration>) -> crate::AppResult<OverlayEvent> {
        self.event(timeout)
    }
    /// 【X11覆盖层】【显示器列表】无参数，返回发现时的物理输出描述
    fn discovered_outputs(&self) -> &[Output] {
        &self.outputs
    }
    /// 【X11覆盖层】【像素网格】无参数，X11采用物理坐标，返回无需额外缩放
    fn hidpi(&self) -> bool {
        false
    }
    /// 【X11覆盖层】【帧状态】按monitor_idx返回表面是否存在，X11提交不等待合成器回调
    fn ready(&self, monitor_idx: usize) -> bool {
        monitor_idx < self.surfaces.len()
    }
    /// 【X11覆盖层】【活动输出】无参数，按根窗口指针位置返回输出索引
    fn active_output(&mut self) -> Option<usize> {
        let pointer = self
            .display
            .connection
            .query_pointer(self.display.screen().root)
            .ok()?
            .reply()
            .ok()?;
        self.outputs.iter().position(|output| {
            let (left, top) = output.info.location;
            let (width, height) = output.info.logical_size.unwrap_or_default();
            i32::from(pointer.root_x) >= left
                && i32::from(pointer.root_x) < left + width
                && i32::from(pointer.root_y) >= top
                && i32::from(pointer.root_y) < top + height
        })
    }
    /// 【X11覆盖层】【输出保留】按keep保留对应表面，返回成功，删除窗口自动回收资源
    fn retain_outputs(&mut self, keep: &[usize]) -> crate::AppResult {
        let mut index = 0;
        self.surfaces.retain(|_| {
            let retain = keep.contains(&index);
            index += 1;
            retain
        });
        Ok(())
    }
    /// 【X11覆盖层】【输出身份】按monitor_idx返回原始输出描述，保持表面裁减后的身份正确
    fn surface_output(&self, monitor_idx: usize) -> Option<&Output> {
        self.surfaces
            .get(monitor_idx)
            .and_then(|(index, _)| self.outputs.get(*index))
    }
    /// 【X11覆盖层】【冻结背景】按索引及尺寸接收旧接口请求；完整合成模式不持有第二份冻结图
    fn set_background(
        &mut self,
        _monitor_idx: usize,
        _pixels: &[u8],
        _width: u32,
        _height: u32,
    ) -> crate::AppResult {
        Ok(())
    }
    /// 【X11覆盖层】【背景清除】按monitor_idx验证目标，完整合成帧由调用方更新
    fn clear_background(&mut self, monitor_idx: usize) -> crate::AppResult {
        self.surfaces
            .get(monitor_idx)
            .ok_or("X11 overlay surface missing")?;
        Ok(())
    }
    /// 【X11覆盖层】【输入区域】按monitor_idx和rect设置Shape输入区域，空值允许事件穿透
    fn set_input_rect(&mut self, monitor_idx: usize, rect: Option<(i32, i32, i32, i32)>) {
        let Some((_, surface)) = self.surfaces.get(monitor_idx) else {
            return;
        };
        let rect = if self.scrolling == Some(monitor_idx) {
            rect.map(|(_, _, width, height)| (0, 0, width, height))
        } else {
            rect
        };
        let rectangles = rect
            .into_iter()
            .filter_map(|(left, top, width, height)| {
                Some(Rectangle {
                    x: left.try_into().ok()?,
                    y: top.try_into().ok()?,
                    width: width.try_into().ok()?,
                    height: height.try_into().ok()?,
                })
            })
            .collect::<Vec<_>>();
        if let Err(error) = self
            .display
            .connection
            .shape_rectangles(
                SO::SET,
                SK::INPUT,
                ClipOrdering::UNSORTED,
                surface.window,
                0,
                0,
                &rectangles,
            )
            .map(|_| ())
        {
            log::error!("【X11覆盖层】【输入区域】{error}");
        }
    }
    /// 【X11覆盖层】【光标设置】按icon创建标准光标字体形状并应用当前表面，无返回值
    fn set_cursor(&mut self, icon: CursorIcon) {
        if let Err(error) = self.cursor(icon) {
            log::info!("【X11覆盖层】【光标设置】{error}");
        }
    }
    /// 【X11覆盖层】【暂停显示】无参数，隐藏全部覆盖窗口，返回协议错误
    fn suspend(&mut self) -> crate::AppResult {
        self.popup = None;
        self.popup_grab = None;
        self.text_focus(None)?;
        for (_, surface) in &mut self.surfaces {
            self.display
                .connection
                .unmap_window(surface.window)?
                .check()?;
            surface.mapped = false;
        }
        self.suspended = true;
        self.display.connection.flush()?;
        Ok(())
    }
    /// 【X11覆盖层】【恢复显示】无参数，恢复已保存帧，返回协议错误
    fn resume(&mut self) -> crate::AppResult {
        for (_, surface) in &mut self.surfaces {
            self.display
                .connection
                .map_window(surface.window)?
                .check()?;
            surface.mapped = true;
            surface.repaint()?;
        }
        self.suspended = false;
        Ok(())
    }
}

impl Overlay {
    /// 【X11覆盖层】【标准光标】按icon应用核心协议光标，返回资源或协议错误
    fn cursor(&self, icon: CursorIcon) -> crate::AppResult {
        super::super::cursor::set(
            &self.display,
            self.surfaces.iter().map(|(_, surface)| surface.window),
            icon,
        )
    }
}
