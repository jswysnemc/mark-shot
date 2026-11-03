//! 【输入法】【事件分派】候选、确认和删除在 done 原子交付，leave 清理候选与旧焦点身份

use crate::{
    backend::wayland::overlay::state::OverlayState,
    types::{OverlayEvent, text_input::Focus},
};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::text_input::zv3::client::{
    zwp_text_input_manager_v3, zwp_text_input_v3,
};

impl Dispatch<zwp_text_input_manager_v3::ZwpTextInputManagerV3, ()> for OverlayState {
    /// 【输入法】【管理器分派】管理器不提供事件，忽略协议参数，无返回值
    fn event(
        _: &mut Self,
        _: &zwp_text_input_manager_v3::ZwpTextInputManagerV3,
        _: zwp_text_input_manager_v3::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwp_text_input_v3::ZwpTextInputV3, ()> for OverlayState {
    /// 【输入法】【对象分派】按 proxy 找到 seat 状态并处理 event，其余参数来自 Wayland；无返回值，未启用目标不能接收正文
    fn event(
        state: &mut Self,
        proxy: &zwp_text_input_v3::ZwpTextInputV3,
        event: zwp_text_input_v3::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(device) = state
            .text_input
            .devices
            .iter_mut()
            .find(|device| &device.proxy == proxy)
        else {
            return;
        };
        match event {
            zwp_text_input_v3::Event::Enter { surface } => {
                device.state.reset();
                device.surface = Some(surface.clone());
                device.focus = state
                    .surfaces
                    .iter()
                    .find(|(_, data)| data.surface == surface)
                    .map(|(monitor_idx, _)| Focus {
                        id: crate::features::text::input_method::focus::identity(),
                        monitor_idx: *monitor_idx,
                    });
                if let Some(focus) = device.focus {
                    state.events.push_back(OverlayEvent::TextInputFocus(focus));
                }
            }
            zwp_text_input_v3::Event::Leave { surface } => {
                if device.surface.as_ref() != Some(&surface) {
                    return;
                }
                if let Some(focus) = device.focus.take() {
                    state
                        .events
                        .push_back(OverlayEvent::TextInputLeave { focus_id: focus.id });
                }
                device.surface = None;
                device.state.reset();
            }
            zwp_text_input_v3::Event::PreeditString {
                text,
                cursor_begin,
                cursor_end,
            } if device.state.current.is_some() => {
                if !device.state.pending.preedit(text, cursor_begin, cursor_end) {
                    log::info!("【输入法】【候选校验】忽略非法 UTF-8 候选光标位置");
                }
            }
            zwp_text_input_v3::Event::CommitString { text } if device.state.current.is_some() => {
                device.state.pending.commit(text)
            }
            zwp_text_input_v3::Event::DeleteSurroundingText {
                before_length,
                after_length,
            } if device.state.current.is_some() => {
                device.state.pending.delete(before_length, after_length)
            }
            zwp_text_input_v3::Event::Done { serial } => {
                if device.state.current.is_some()
                    && let Some(target_id) = device.state.target_for(serial)
                {
                    let update =
                        device
                            .state
                            .pending
                            .done(target_id, serial, device.state.commit_count);
                    state.events.push_back(OverlayEvent::TextInput(update));
                } else {
                    device.state.pending.reset();
                }
            }
            _ => {}
        }
    }
}
