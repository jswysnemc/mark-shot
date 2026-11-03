//! 【输入法】【共用消息】按表面归属交付原子编辑，不依赖截图或独立窗口的业务类型

use super::TextInput;
use crate::types::text_input::{Focus, Update};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_protocols::wp::text_input::zv3::client::zwp_text_input_v3::{Event, ZwpTextInputV3};

pub(crate) enum Message {
    Focus(Focus),
    Leave(u64),
    Edit(Update),
}

impl TextInput {
    /// 【输入法】【消息归属】处理 proxy 的 event，resolve 为进入表面分配原生焦点；返回业务消息，非法或未启用输入返回空值
    pub(crate) fn message(
        &mut self,
        proxy: &ZwpTextInputV3,
        event: Event,
        resolve: impl FnOnce(&WlSurface) -> Option<Focus>,
    ) -> Option<Message> {
        let device = self
            .devices
            .iter_mut()
            .find(|device| &device.proxy == proxy)?;
        match event {
            Event::Enter { surface } => {
                device.state.reset();
                device.focus = resolve(&surface);
                device.surface = Some(surface);
                device.focus.map(Message::Focus)
            }
            Event::Leave { surface } if device.surface.as_ref() == Some(&surface) => {
                let focus = device.focus.take();
                device.surface = None;
                device.state.reset();
                focus.map(|focus| Message::Leave(focus.id))
            }
            Event::PreeditString {
                text,
                cursor_begin,
                cursor_end,
            } if device.state.current.is_some() => {
                if !device.state.pending.preedit(text, cursor_begin, cursor_end) {
                    log::info!("【输入法】【候选校验】忽略非法 UTF-8 候选光标位置");
                }
                None
            }
            Event::CommitString { text } if device.state.current.is_some() => {
                device.state.pending.commit(text);
                None
            }
            Event::DeleteSurroundingText {
                before_length,
                after_length,
            } if device.state.current.is_some() => {
                device.state.pending.delete(before_length, after_length);
                None
            }
            Event::Done { serial } => {
                if device.state.current.is_some()
                    && let Some(target_id) = device.state.target_for(serial)
                {
                    Some(Message::Edit(device.state.pending.done(
                        target_id,
                        serial,
                        device.state.commit_count,
                    )))
                } else {
                    device.state.pending.reset();
                    None
                }
            }
            _ => None,
        }
    }
}
