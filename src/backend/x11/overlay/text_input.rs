//! 【X11覆盖层】【输入法衔接】将窗口焦点与正文身份同步给XIM，旧候选不得进入新正文

use super::*;
use crate::{
    backend::x11::input_method::Message,
    types::text_input::{Batch, Focus, Snapshot, Update},
};
use x11rb::protocol::{Event, xproto::KEY_PRESS_EVENT};

impl Overlay {
    /// 【X11覆盖层】【输入焦点】按monitor记录新的输入法焦点，空值撤销旧正文并排入离开通知
    pub(super) fn text_focus(&mut self, monitor: Option<usize>) -> crate::AppResult {
        if let Some(focus) = self.text_focus.take() {
            self.pending
                .push_back(OverlayEvent::TextInputLeave { focus_id: focus.id });
        }
        self.text_snapshot = None;
        self.ime.sync(0, None)?;
        self.keyboard.reset();
        if let Some(monitor_idx) = monitor {
            let focus = Focus {
                id: crate::features::text::input_method::focus::identity(),
                monitor_idx,
            };
            self.text_focus = Some(focus);
            self.pending.push_back(OverlayEvent::TextInputFocus(focus));
        }
        Ok(())
    }

    /// 【X11覆盖层】【正文快照】按snapshot同步当前焦点的正文和候选位置，返回协议错误
    pub(super) fn sync_text_input(&mut self, snapshot: Option<&Snapshot>) -> crate::AppResult {
        let snapshot = snapshot.filter(|snapshot| {
            self.text_focus.is_some_and(|focus| {
                focus.id == snapshot.focus_id && focus.monitor_idx == snapshot.monitor_idx
            })
        });
        let window = snapshot
            .and_then(|snapshot| self.surfaces.get(snapshot.monitor_idx))
            .map_or(0, |(_, surface)| surface.window);
        if self
            .text_snapshot
            .as_ref()
            .map(|snapshot| snapshot.target_id)
            != snapshot.map(|snapshot| snapshot.target_id)
        {
            self.keyboard.reset();
        }
        self.ime.sync(window, snapshot)?;
        self.text_snapshot = snapshot.cloned();
        Ok(())
    }

    /// 【X11覆盖层】【输入法回调】无参数，交付当前正文候选及未处理按键，返回协议错误
    pub(super) fn collect_ime(&mut self) -> crate::AppResult {
        while let Some(message) = self.ime.pop() {
            let target = match &message {
                Message::Key { target, .. } | Message::Edit { target, .. } => *target,
            };
            if !self
                .text_snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.target_id == target)
            {
                continue;
            }
            match message {
                Message::Edit { target, edit } => {
                    self.pending.push_back(OverlayEvent::TextInput(Update {
                        target_id: target,
                        serial: 0,
                        sync_allowed: true,
                        batch: Batch {
                            preedit: edit.preedit,
                            commit: edit.commit,
                            ..Default::default()
                        },
                    }))
                }
                Message::Key { event, .. } => {
                    let event = if event.response_type & 0x7f == KEY_PRESS_EVENT {
                        Event::KeyPress(event)
                    } else {
                        Event::KeyRelease(event)
                    };
                    if let Some(event) = self.translate(event)? {
                        self.pending.push_back(event);
                    }
                }
            }
        }
        Ok(())
    }
}
