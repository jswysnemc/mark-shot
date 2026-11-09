//! 【X11普通窗口】【正文衔接】将共享内容的字段快照连接XIM，模态和菜单期间暂停父正文

use super::*;
use crate::{
    backend::x11::input_method::Message,
    types::text_input::{ChangeCause, Focus},
};

impl Node {
    /// 【X11普通窗口】【字段同步】无参数，按真实焦点及字段身份同步输入法，不向隐藏正文交付候选
    pub(super) fn sync_text_input(&mut self) -> crate::AppResult {
        let target = self
            .focus
            .filter(|_| self.dialog.is_none() && self.popups.is_empty())
            .and_then(|focus| {
                self.content
                    .text_input_target()
                    .map(|target| (focus, target))
            });
        let identity = target.map(|(focus, target)| (focus.id, target));
        if self
            .text_snapshot
            .as_ref()
            .map(|snapshot| (snapshot.focus_id, snapshot.target_id))
            != identity
        {
            self.keyboard.reset();
            self.dirty |= self.content.cancel_text_input();
            self.text_snapshot = None;
            self.ime.sync(0, None)?;
        }
        if let Some((focus, target)) = target
            && let Some(snapshot) = self.content.text_input_snapshot(
                focus,
                target,
                self.text_snapshot.as_ref(),
                self.text_cause,
            )
        {
            self.ime.sync(self.surface.window, Some(&snapshot))?;
            self.text_snapshot = Some(snapshot);
            self.text_cause = ChangeCause::Other;
        }
        Ok(())
    }

    /// 【X11普通窗口】【焦点切换】按focused建立或撤销输入身份，返回协议结果
    pub(super) fn text_focus(&mut self, focused: bool) -> crate::AppResult {
        self.focus = focused.then(|| Focus {
            id: crate::features::text::input_method::focus::identity(),
            monitor_idx: 0,
        });
        self.sync_text_input()
    }

    /// 【X11普通窗口】【协议事件】按event处理XIM消息，返回是否消费；原始按键通过独立入口避免再次转发
    pub(super) fn ime_event(&mut self, event: &Event) -> crate::AppResult<bool> {
        if !self.ime.filter(event) {
            return Ok(false);
        }
        while let Some(message) = self.ime.pop() {
            let target = match &message {
                Message::Key { target, .. } | Message::Edit { target, .. } => *target,
            };
            if self.dialog.is_some()
                || !self.popups.is_empty()
                || self.content.text_input_target() != Some(target)
            {
                continue;
            }
            let event = match message {
                Message::Edit { edit, .. } => {
                    self.text_cause = ChangeCause::InputMethod;
                    WindowEvent::TextInput(edit)
                }
                Message::Key { event, .. } => {
                    let pressed = event.response_type & 0x7f == KEY_PRESS_EVENT;
                    let input = self.keyboard.key(&event, pressed);
                    if pressed {
                        WindowEvent::Key(input)
                    } else {
                        WindowEvent::KeyRelease(input)
                    }
                }
            };
            let action = self.content.event(event);
            self.action(action);
        }
        Ok(true)
    }
}
