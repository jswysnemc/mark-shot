//! 【Windows覆盖层】【帧提交】每输出只复用一个DIB，按损坏范围合成原始冻结来源

use super::*;
use crate::{
    backend::ScreenOverlay,
    image_source::ImageSource,
    types::{CursorIcon, DamageRect},
};
use std::{io, time::Duration};

impl ScreenOverlay for Overlay {
    /// 【Windows覆盖层】【背景能力】无参数，返回原生BGRA合成可用，不分配独立完整背景
    fn supports_native_frame_background(&self) -> bool {
        true
    }
    /// 【Windows覆盖层】【背景模式】按enabled设置共享合成策略，返回实际启用状态
    fn set_composited_background(&mut self, enabled: bool) -> bool {
        self.composited = enabled;
        enabled
    }
    /// 【Windows覆盖层】【原生合成】按monitor_idx、pixels、background与damage更新唯一DIB，返回结果，原图仍由业务拥有
    fn stage_frame_with_background(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
        background: &ImageSource,
        damage: Option<DamageRect>,
    ) -> crate::AppResult {
        let size = (background.width(), background.height());
        self.surface(monitor_idx)?.update_bitmap(size, |bitmap| {
            bitmap.write_rgba_region(pixels, damage)?;
            crate::backend::native_backdrop::compose_bgra(
                background,
                bitmap.pixels_mut(),
                size,
                damage,
            )
            .map_err(|error| io::Error::other(error.to_string()))
        })?;
        self.staged[monitor_idx] = true;
        Ok(())
    }
    /// 【Windows覆盖层】【预先准备】无参数，保持屏幕缓冲按首帧需求分配，无返回值
    fn prefault(&mut self) {}
    /// 【Windows覆盖层】【线程内创建】按targets创建隐藏全屏窗口，返回结果；像素交付前不显示
    fn present(&mut self, targets: &[usize]) -> crate::AppResult {
        self.popup = None;
        self.surfaces.clear();
        for &index in targets {
            let output = self
                .outputs
                .get(index)
                .ok_or("invalid Windows display target")?;
            let window = native::Window::new(
                native::WindowOptions {
                    title: "Mark Shot".into(),
                    bounds: output.native.bounds,
                    minimum: (1, 1),
                    topmost: true,
                    activate: true,
                    layered: false,
                    owner: 0,
                },
                self.queue.clone(),
            )?;
            self.surfaces.push((index, window));
        }
        self.staged = vec![false; self.surfaces.len()];
        Ok(())
    }
    /// 【Windows覆盖层】【前景更新】按monitor_idx、pixels与damage复制RGBA，返回结果，不重复创建位图
    fn stage_frame(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
        damage: Option<DamageRect>,
    ) -> crate::AppResult {
        let bounds = self.surface(monitor_idx)?.bounds()?;
        self.surface(monitor_idx)?
            .update_bitmap((bounds.width, bounds.height), |bitmap| {
                bitmap.write_rgba_region(pixels, damage)
            })?;
        self.staged[monitor_idx] = true;
        Ok(())
    }
    /// 【Windows覆盖层】【批量呈现】无参数，只提交本轮变化表面，返回系统绘制结果，空闲不绘制
    fn flush(&mut self) -> crate::AppResult {
        for (index, (_, window)) in self.surfaces.iter_mut().enumerate() {
            if self.staged[index] {
                window.present()?;
                self.staged[index] = false;
            }
        }
        Ok(())
    }
    /// 【Windows覆盖层】【共同等待】按timeout等待窗口、主题和后台来源，返回共享输入事件，不创建轮询线程
    fn next_event(&mut self, timeout: Option<Duration>) -> crate::AppResult<OverlayEvent> {
        self.wait_event(timeout)
    }
    /// 【Windows覆盖层】【输出列表】无参数，返回初次发现的显示器描述，不复制屏幕像素
    fn discovered_outputs(&self) -> &[Output] {
        &self.outputs
    }
    /// 【Windows覆盖层】【物理网格】无参数，返回物理坐标模式，界面与捕获共用同一原生网格
    fn hidpi(&self) -> bool {
        false
    }
    /// 【Windows覆盖层】【呈现状态】按monitor_idx返回是否存在可用原生表面，同步DIB提交不等待合成器回调
    fn ready(&self, monitor_idx: usize) -> bool {
        monitor_idx < self.surfaces.len()
    }
    /// 【Windows覆盖层】【活动显示器】无参数，按当前系统指针返回显示器索引，读取失败时返回主显示器
    fn active_output(&mut self) -> Option<usize> {
        use windows_sys::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};
        let mut point = POINT { x: 0, y: 0 };
        if unsafe { GetCursorPos(&mut point) } != 0 {
            self.outputs.iter().position(|output| {
                let bounds = output.native.bounds;
                point.x >= bounds.left
                    && point.y >= bounds.top
                    && i64::from(point.x) < i64::from(bounds.left) + i64::from(bounds.width)
                    && i64::from(point.y) < i64::from(bounds.top) + i64::from(bounds.height)
            })
        } else {
            self.outputs.iter().position(|output| output.native.primary)
        }
    }
    /// 【Windows覆盖层】【输出保留】按keep回收未捕获窗口，返回索引校验结果，已有HWND不跨线程移动
    fn retain_outputs(&mut self, keep: &[usize]) -> crate::AppResult {
        if keep.iter().any(|index| *index >= self.surfaces.len()) {
            return Err("invalid retained Windows surface".into());
        }
        let mut previous = std::mem::take(&mut self.surfaces)
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>();
        for &index in keep {
            self.surfaces.push(
                previous[index]
                    .take()
                    .ok_or("duplicate retained Windows surface")?,
            );
        }
        self.staged = vec![false; self.surfaces.len()];
        Ok(())
    }
    /// 【Windows覆盖层】【冻结安装】按monitor_idx与原始pixels验证背景尺寸，返回结果；合成路径由每帧借用原图
    fn set_background(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> crate::AppResult {
        if !self.composited {
            return Err("Windows capture requires composited frozen frames".into());
        }
        let bounds = self.surface(monitor_idx)?.bounds()?;
        if (bounds.width, bounds.height) != (width, height)
            || pixels.len() as u64 != u64::from(width) * u64::from(height) * 4
        {
            return Err("Windows frozen frame does not match its output".into());
        }
        Ok(())
    }
    /// 【Windows覆盖层】【来源安装】按monitor_idx和source验证原图网格，返回结果，不把映射文件完整读取到驻留内存
    fn set_background_source(
        &mut self,
        monitor_idx: usize,
        source: &ImageSource,
    ) -> crate::AppResult {
        let bounds = self.surface(monitor_idx)?.bounds()?;
        if (bounds.width, bounds.height) != (source.width(), source.height()) {
            return Err("Windows source does not match its output".into());
        }
        Ok(())
    }
    /// 【Windows覆盖层】【光标同步】按icon更新全部表面指针，无返回值，加载失败记录系统错误
    fn set_cursor(&mut self, icon: CursorIcon) {
        let cursor = match icon {
            CursorIcon::Default => native::Cursor::Default,
            CursorIcon::Crosshair => native::Cursor::Crosshair,
            CursorIcon::Pointer => native::Cursor::Pointer,
            CursorIcon::Progress => native::Cursor::Wait,
            CursorIcon::Text => native::Cursor::Text,
            CursorIcon::Grab | CursorIcon::Grabbing => native::Cursor::Move,
            CursorIcon::NsResize => native::Cursor::VerticalResize,
            CursorIcon::EwResize => native::Cursor::HorizontalResize,
            CursorIcon::NeswResize => native::Cursor::NeSwResize,
            CursorIcon::NwseResize => native::Cursor::NwSeResize,
        };
        for (_, window) in &mut self.surfaces {
            if let Err(error) = window.set_cursor(cursor) {
                log::error!("【Windows覆盖层】【光标同步】{error}");
            }
        }
    }
    /// 【Windows覆盖层】【冻结移除】按monitor_idx隐藏对应窗口，返回系统表面查询结果，原图来源仍由编辑器保存
    fn clear_background(&mut self, monitor_idx: usize) -> crate::AppResult {
        self.surface(monitor_idx)?.hide();
        Ok(())
    }
    /// 【Windows覆盖层】【输入范围】按monitor_idx和rect接收共享输入配置，无返回值；滚动角色将在独立控制窗口实现
    fn set_input_rect(&mut self, _monitor_idx: usize, _rect: Option<(i32, i32, i32, i32)>) {}
    /// 【Windows覆盖层】【输出绑定】按monitor_idx返回窗口创建时的原生显示器，超界返回空值
    fn surface_output(&self, monitor_idx: usize) -> Option<&Output> {
        self.surfaces
            .get(monitor_idx)
            .and_then(|entry| self.outputs.get(entry.0))
    }
    /// 【Windows覆盖层】【任务唤醒】无参数，返回现有后台通知句柄，不创建新消息线程
    fn task_wake(&mut self) -> crate::AppResult<task_wake::Wake> {
        Ok(self.wake.wake())
    }
    /// 【Windows覆盖层】【隐藏暂停】无参数，先回收弹层再隐藏截图表面，返回结果，不释放原图
    fn suspend(&mut self) -> crate::AppResult {
        self.popup = None;
        for (_, window) in &mut self.surfaces {
            window.hide();
        }
        Ok(())
    }
    /// 【Windows覆盖层】【首帧恢复】无参数，等待共享业务重新提交全部像素，返回结果，避免显示暂停前缓存
    fn resume(&mut self) -> crate::AppResult {
        Ok(())
    }
    /// 【Windows覆盖层】【正文同步】按snapshot同步原生输入法目标和光标，空值取消旧候选，返回系统结果
    fn set_text_input(
        &mut self,
        snapshot: Option<&crate::types::text_input::Snapshot>,
    ) -> crate::AppResult {
        for (index, (_, window)) in self.surfaces.iter_mut().enumerate() {
            let target = snapshot.filter(|snapshot| snapshot.monitor_idx == index);
            window.set_text_input(
                target.map_or(0, |snapshot| snapshot.target_id),
                target.and_then(|snapshot| snapshot.cursor_rect),
            )?;
        }
        self.text = snapshot.cloned();
        Ok(())
    }
    /// 【Windows覆盖层】【菜单同步】按frame创建或更新自绘原生弹层，空值销毁，返回原生菜单可用状态
    fn set_popup(
        &mut self,
        frame: Option<crate::types::popup::PopupFrame<'_>>,
    ) -> crate::AppResult<bool> {
        let Some(frame) = frame else {
            self.popup = None;
            return Ok(true);
        };
        crate::types::popup::validate(&frame)?;
        if self.popup.as_ref().is_none_or(|entry| entry.0 != frame.id) {
            self.popup = None;
            let parent = self.surface(frame.monitor_idx)?;
            let bounds = parent.bounds()?;
            let owner = parent.handle();
            let window = native::Window::new(
                native::WindowOptions {
                    title: "Mark Shot menu".into(),
                    bounds: native::Bounds {
                        left: bounds.left + (frame.anchor.0 * frame.scale).round() as i32,
                        top: bounds.top + (frame.anchor.1 * frame.scale).round() as i32,
                        width: frame.pixel_size.0,
                        height: frame.pixel_size.1,
                    },
                    minimum: (1, 1),
                    topmost: true,
                    activate: false,
                    layered: true,
                    owner,
                },
                self.queue.clone(),
            )?;
            self.pending.push_back(OverlayEvent::Popup {
                id: frame.id,
                event: crate::types::popup::PopupEvent::Configured {
                    position: (frame.anchor.0.round() as i32, frame.anchor.1.round() as i32),
                },
            });
            self.popup = Some((frame.id, frame.monitor_idx, window));
        }
        self.popup
            .as_mut()
            .unwrap()
            .2
            .set_rgba(frame.pixel_size, frame.pixels)?;
        Ok(true)
    }
}
