//! 【输入法】【事件分派】候选、确认和删除在 done 原子交付，leave 清理候选与旧焦点身份

use super::message::Message;
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
        let message = state.text_input.message(proxy, event, |surface| {
            state
                .surfaces
                .iter()
                .find(|(_, data)| &data.surface == surface)
                .map(|(monitor_idx, _)| Focus {
                    id: crate::features::text::input_method::focus::identity(),
                    monitor_idx: *monitor_idx,
                })
        });
        if let Some(message) = message {
            state.events.push_back(match message {
                Message::Focus(focus) => OverlayEvent::TextInputFocus(focus),
                Message::Leave(focus_id) => OverlayEvent::TextInputLeave { focus_id },
                Message::Edit(update) => OverlayEvent::TextInput(update),
            });
        }
    }
}
