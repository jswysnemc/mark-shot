//! 【截图弹层】【原生生命周期】真实按钮序列号建立 xdg-popup 抓取，子表面关闭不结束截图或文字事务

use crate::{
    AppResult,
    backend::wayland::{
        overlay::state::OverlayState,
        utils::{buffers, surface::Role},
    },
    types::{
        OverlayEvent,
        popup::{PopupEvent, PopupFrame, validate},
    },
};
use smithay_client_toolkit::{
    compositor::FrameCallbackData,
    shell::xdg::{
        XdgPositioner, XdgSurface,
        popup::{Popup, PopupConfigure, PopupHandler},
    },
    shm::slot::Buffer,
};
use wayland_client::{Connection, QueueHandle};
use wayland_protocols::{
    wp::viewporter::client::wp_viewport::WpViewport,
    xdg::shell::client::xdg_positioner::{Anchor, ConstraintAdjustment, Gravity},
};

pub struct OverlayPopup {
    pub id: u64,
    pub popup: Popup,
    pub configured: bool,
    pub keyboard_focused: bool,
    pub position: (i32, i32),
    pub monitor_idx: usize,
    buffers: Vec<Buffer>,
    viewport: Option<WpViewport>,
    scale: f32,
    pixel_size: (u32, u32),
    pixels: Vec<u8>,
    dirty: bool,
    frame_pending: bool,
}

/// 【截图弹层】【按下记录清理】从 records 清除 serial 之前已经分派的按下记录，保留当前和未来输入；支持序列号回绕，无返回值
pub fn retain_current_press<T>(records: &mut std::collections::VecDeque<(u32, T)>, serial: u32) {
    if let Some(index) = records.iter().position(|(value, _)| *value == serial) {
        records.drain(..index);
    }
}

impl Drop for OverlayPopup {
    /// 【截图弹层】【资源释放】先销毁视口，再由工具包释放弹出角色与表面，无参数和返回值
    fn drop(&mut self) {
        if let Some(viewport) = self.viewport.take() {
            viewport.destroy();
        }
    }
}

impl OverlayState {
    /// 【截图弹层】【输入交付】将 event 与当前弹层编号加入事件队列，不改变主画布指针或字符选区，无返回值
    pub fn popup_event(&mut self, event: PopupEvent) {
        if let Some(popup) = &self.popup {
            self.events.push_back(OverlayEvent::Popup {
                id: popup.id,
                event,
            });
        }
    }

    /// 【截图弹层】【关闭通知】先通知上层再释放原生抓取；不产生截图退出事件，无参数和返回值
    pub fn dismiss_popup(&mut self) {
        self.popup_event(PopupEvent::Closed);
        self.popup = None;
        self.pending_flush = true;
    }

    /// 【截图弹层】【创建更新】按 frame 与 qh 创建或更新菜单；序列号必须属于真实按钮及同一座席，返回协议结果
    pub fn sync_popup(&mut self, frame: PopupFrame<'_>, qh: &QueueHandle<Self>) -> AppResult {
        validate(&frame)?;
        if self.viewporter.is_none() && (frame.scale < 1.0 || frame.scale.fract() != 0.0) {
            return Err("fractional popup scale requires the viewporter protocol".into());
        }
        if self.popup.as_ref().is_none_or(|popup| popup.id != frame.id) {
            self.popup = None;
            let serial = frame.serial.ok_or("popup has no pointer press serial")?;
            let seat = self
                .popup_grabs
                .iter()
                .find(|(value, _)| *value == serial)
                .map(|(_, seat)| seat.clone())
                .ok_or("popup serial does not belong to a recorded press")?;
            let parent = self
                .surfaces
                .get(&frame.monitor_idx)
                .ok_or("popup parent output is absent")?;
            let positioner = XdgPositioner::new(&self.xdg_shell)?;
            positioner.set_size(frame.size.0 as i32, frame.size.1 as i32);
            positioner.set_anchor_rect(
                frame.anchor.0.round() as i32,
                frame.anchor.1.round() as i32,
                1,
                1,
            );
            // 1. 【截图弹层】【锚点对齐】面板左上角等于输入点，避免一像素矩形的右下角产生额外位移
            positioner.set_anchor(Anchor::TopLeft);
            positioner.set_gravity(Gravity::BottomRight);
            positioner.set_constraint_adjustment(
                ConstraintAdjustment::SlideX
                    | ConstraintAdjustment::SlideY
                    | ConstraintAdjustment::FlipX
                    | ConstraintAdjustment::FlipY,
            );
            let xdg_parent = match &parent.role {
                Role::Window(window) => Some(window.xdg_surface()),
                Role::Layer(_) => None,
            };
            let popup = Popup::from_surface(
                xdg_parent,
                &positioner,
                qh,
                self.compositor_state.create_surface(qh),
                &self.xdg_shell,
            )?;
            if let Role::Layer(layer) = &parent.role {
                layer.get_popup(popup.xdg_popup());
            }
            popup.xdg_popup().grab(&seat, serial);
            let viewport = self.viewporter.as_ref().map(|manager| {
                let viewport = manager.get_viewport(popup.wl_surface(), qh, ());
                viewport.set_destination(frame.size.0 as i32, frame.size.1 as i32);
                viewport
            });
            popup.wl_surface().commit();
            self.popup = Some(OverlayPopup {
                id: frame.id,
                popup,
                configured: false,
                keyboard_focused: false,
                position: (0, 0),
                monitor_idx: frame.monitor_idx,
                buffers: Vec::new(),
                viewport,
                scale: frame.scale,
                pixel_size: frame.pixel_size,
                pixels: Vec::new(),
                dirty: true,
                frame_pending: false,
            });
        }
        let popup = self.popup.as_mut().unwrap();
        if popup.scale != frame.scale || popup.pixel_size != frame.pixel_size {
            return Err("popup dimensions changed during a grab".into());
        }
        popup.pixels.clear();
        popup.pixels.extend_from_slice(frame.pixels);
        popup.dirty = true;
        self.paint_popup(qh)
    }

    /// 【截图弹层】【像素提交】用 qh 提交最新菜单缓冲；等待配置和帧回调时保留最新像素，不启动空闲轮询，返回绘制结果
    pub fn paint_popup(&mut self, qh: &QueueHandle<Self>) -> AppResult {
        let Some(popup) = self.popup.as_mut() else {
            return Ok(());
        };
        if !popup.configured || !popup.dirty || popup.frame_pending {
            return Ok(());
        }
        let Some(index) = buffers::available(&mut self.pool, &mut popup.buffers, popup.pixel_size)?
        else {
            return Ok(());
        };
        let buffer = &popup.buffers[index];
        let canvas = buffer
            .canvas(&mut self.pool)
            .ok_or("popup buffer is active")?;
        crate::utils::copy_swizzled(canvas, &popup.pixels);
        if popup.viewport.is_none() {
            popup
                .popup
                .wl_surface()
                .set_buffer_scale(popup.scale as i32);
        }
        buffer.attach_to(popup.popup.wl_surface())?;
        popup.popup.wl_surface().damage_buffer(
            0,
            0,
            popup.pixel_size.0 as i32,
            popup.pixel_size.1 as i32,
        );
        popup
            .popup
            .wl_surface()
            .frame(qh, FrameCallbackData(popup.popup.wl_surface().clone()));
        popup.popup.wl_surface().commit();
        popup.frame_pending = true;
        popup.dirty = false;
        self.pending_flush = true;
        Ok(())
    }

    /// 【截图弹层】【帧完成】判断 surface 是否为当前弹层，若是则提交挂起像素；qh 绑定回调，返回是否消费该帧
    pub fn popup_frame(
        &mut self,
        surface: &wayland_client::protocol::wl_surface::WlSurface,
        qh: &QueueHandle<Self>,
    ) -> bool {
        let Some(popup) = self
            .popup
            .as_mut()
            .filter(|popup| popup.popup.wl_surface() == surface)
        else {
            return false;
        };
        popup.frame_pending = false;
        if let Err(error) = self.paint_popup(qh) {
            log::error!("【截图弹层】【帧提交】{error}");
            self.dismiss_popup();
        }
        true
    }
}

impl PopupHandler for OverlayState {
    /// 【截图弹层】【首次配置】按 popup、configure 与 qh 接受位置并提交缓冲；失败只关闭弹层，不退出截图，无返回值
    fn configure(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        popup: &Popup,
        configure: PopupConfigure,
    ) {
        let Some(current) = self
            .popup
            .as_mut()
            .filter(|current| &current.popup == popup)
        else {
            return;
        };
        current.configured = true;
        current.position = configure.position;
        self.popup_event(PopupEvent::Configured {
            position: configure.position,
        });
        if let Err(error) = self.paint_popup(qh) {
            log::error!("【截图弹层】【首次配置】{error}");
            self.dismiss_popup();
        }
    }
    /// 【截图弹层】【合成器取消】仅关闭 popup 对应的当前菜单，过期通知不影响新会话，无返回值
    fn done(&mut self, _: &Connection, _: &QueueHandle<Self>, popup: &Popup) {
        if self
            .popup
            .as_ref()
            .is_some_and(|current| &current.popup == popup)
        {
            self.dismiss_popup();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【截图弹层测试】【排队序列号】未来密集输入不挤掉当前抓取，分派后只保留当前及后续记录；序列号回绕不改变顺序，无参数、无返回值
    #[test]
    fn queued_future_presses_do_not_discard_the_current_grab_serial() {
        let mut records = (1..=64).map(|serial| (serial, ())).collect();
        retain_current_press(&mut records, 1);
        assert_eq!(records.len(), 64);
        retain_current_press(&mut records, 32);
        assert_eq!(records.front().unwrap().0, 32);
        assert_eq!(records.len(), 33);
        retain_current_press(&mut records, 64);
        assert_eq!(records.len(), 1);
        let mut wrapped = std::collections::VecDeque::from([(u32::MAX, ()), (0, ()), (1, ())]);
        retain_current_press(&mut wrapped, 0);
        assert_eq!(wrapped.front().unwrap().0, 0);
        retain_current_press(&mut wrapped, 100);
        assert_eq!(wrapped.len(), 2);
    }

    /// 【截图弹层测试】【帧参数】返回 id、scale、pixels 对应的二十像素测试帧，不连接合成器
    fn frame(id: u64, scale: f32, pixels: &[u8]) -> PopupFrame<'_> {
        PopupFrame {
            id,
            monitor_idx: 0,
            serial: Some(1),
            anchor: (0.0, 0.0),
            size: (20, 20),
            scale,
            pixels,
            pixel_size: ((20.0 * scale).ceil() as u32, (20.0 * scale).ceil() as u32),
        }
    }

    /// 【截图弹层测试】【合法缩放】一倍、分数与两倍帧通过严格长度校验，错误长度、零编号和无效比例拒绝，无参数、无返回值
    #[test]
    fn frames_validate_fractional_dimensions_and_reject_invalid_payloads() {
        for scale in [0.5, 0.75, 1.0, 1.2, 2.0] {
            let edge = (20.0_f32 * scale).ceil() as usize;
            let pixels = vec![0; edge * edge * 4];
            assert!(validate(&frame(1, scale, &pixels)).is_ok());
            assert!(validate(&frame(0, scale, &pixels)).is_err());
            assert!(validate(&frame(1, scale, &pixels[..pixels.len() - 1])).is_err());
        }
        for scale in [-1.0, 0.0, 17.0, f32::NAN, f32::INFINITY] {
            assert!(validate(&frame(1, scale, &[])).is_err());
        }
    }
}
