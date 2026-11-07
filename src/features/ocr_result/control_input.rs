//! 【文字识别】【按钮事务】区分正文、父窗口、页签和按钮焦点，按钮动作只由完成的释放交付

use super::{
    layout::Control,
    window::{ResultWindow, View},
};
use crate::backend::window::{KeyInput, LEFT_BUTTON, WindowAction};
use crate::keys::Key;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum FocusTarget {
    #[default]
    Content,
    Window,
    Tabs,
    Button(Control),
    Translation,
    Language,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Press {
    Pointer(Control),
    Keyboard(Control),
}

impl ResultWindow {
    /// 【文字识别】【按钮资格】返回 control 当前是否可执行；空白原文不启用复制，撤销使用局部历史
    pub(super) fn control_enabled(&self, control: Control) -> bool {
        match control {
            Control::Undo => self.can_undo(),
            Control::Copy => self.source.has_non_whitespace(),
            Control::TableCopy => self.table.is_some() && self.view == View::Table,
            Control::TranslationToggle => {
                self.source.has_non_whitespace()
                    || self.translation.state.visible
                    || !self.translation.editor.is_empty()
            }
            Control::Translate => self.translation.state.busy() || self.source.has_non_whitespace(),
            Control::TranslationCopy => self.translation.editor.has_non_whitespace(),
            Control::TranslationUndo => {
                !self.translation.state.busy() && self.translation.editor.can_undo()
            }
            Control::LanguageMenu => !self.translation.state.busy(),
            Control::AlwaysOnTop => self.stacking.supported,
            _ => true,
        }
    }

    /// 【文字识别】【按钮按下外观】返回 control 的可见按下状态，指针移出暂停高亮，键盘按下不依赖悬停
    pub(super) fn control_down(&self, control: Control) -> bool {
        match self.control_press {
            Some(Press::Pointer(pressed)) => pressed == control && self.hovered == Some(control),
            Some(Press::Keyboard(pressed)) => pressed == control,
            None => false,
        }
    }

    /// 【文字识别】【焦点切换】把焦点改为 target，取消旧按钮和候选事务；返回是否发生变化，不修改确认正文
    pub(super) fn focus_control(&mut self, target: FocusTarget) -> bool {
        self.change_focus(target, false)
    }

    /// 【文字识别】【焦点拥有者】按 target 切换输入，keep_text_preedit 仅用于页签切换，返回是否发生变化；保留候选不等同于保留协议资格
    fn change_focus(&mut self, target: FocusTarget, keep_text_preedit: bool) -> bool {
        if self.focus_target == target {
            return false;
        }
        if self.view == View::Text
            && matches!(
                self.focus_target,
                FocusTarget::Content | FocusTarget::Translation
            )
        {
            self.text_focus = self.focus_target;
        }
        if self.focus_target == FocusTarget::Language
            && target != FocusTarget::Language
            && !self.translation.state.busy()
        {
            super::translation::controller::commit_target(self);
        }
        self.focus_target = target;
        self.control_press = None;
        self.dragging = false;
        if let Some(table) = &mut self.table {
            table.bars.cancel();
        }
        self.preview.release();
        self.last_click = None;
        if keep_text_preedit {
            super::input_method::cancel_non_text(self);
        } else {
            super::input_method::cancel(self);
        }
        true
    }

    /// 【文字识别】【控件按下】按 control、button 处理焦点与事务，页签左键按下换页，其余按钮等待释放，返回窗口动作
    pub(super) fn press_control(&mut self, control: Control, button: u32) -> WindowAction {
        if let Control::View(_) = control {
            self.change_focus(FocusTarget::Tabs, true);
            return if button == LEFT_BUTTON {
                self.control(control)
            } else {
                WindowAction::Redraw
            };
        }
        if !self.control_enabled(control) {
            self.focus_control(FocusTarget::Window);
            return WindowAction::Redraw;
        }
        self.focus_control(FocusTarget::Button(control));
        self.hovered = Some(control);
        self.dragging = false;
        self.last_click = None;
        if button == LEFT_BUTTON {
            self.control_press = Some(Press::Pointer(control));
        }
        WindowAction::Redraw
    }

    /// 【文字识别】【指针释放】在 position 完成本次指针事务，仅原按钮内部释放交付动作；没有事务返回空动作
    pub(super) fn release_control(&mut self, position: (f64, f64)) -> WindowAction {
        let Some(Press::Pointer(control)) = self.control_press else {
            return WindowAction::None;
        };
        self.control_press = None;
        if self.layout.control(position, self.view) == Some(control)
            && self.control_enabled(control)
        {
            self.finish_control(control)
        } else {
            WindowAction::Redraw
        }
    }

    /// 【文字识别】【按钮按键】按 input 更新按钮 Space 状态，返回已消费动作；普通 Return 不执行非默认按钮
    pub(super) fn control_key(&mut self, input: &KeyInput) -> Option<WindowAction> {
        let FocusTarget::Button(control) = self.focus_target else {
            return (!matches!(
                self.focus_target,
                FocusTarget::Content | FocusTarget::Translation | FocusTarget::Language
            ))
            .then_some(WindowAction::None);
        };
        if matches!(input.key, Some(Key::Space | Key::Char(' '))) && self.control_enabled(control) {
            if self.control_press == Some(Press::Keyboard(control)) {
                return Some(WindowAction::None);
            }
            self.control_press = Some(Press::Keyboard(control));
            return Some(WindowAction::Redraw);
        }
        Some(WindowAction::None)
    }

    /// 【文字识别】【按钮键盘释放】用 input 完成 Space 事务，仅当前焦点仍属于启用按钮时交付，返回窗口动作
    pub(super) fn release_control_key(&mut self, input: &KeyInput) -> WindowAction {
        if !matches!(input.key, Some(Key::Space | Key::Char(' '))) {
            return WindowAction::None;
        }
        let Some(Press::Keyboard(control)) = self.control_press else {
            return WindowAction::None;
        };
        self.control_press = None;
        if self.focus_target == FocusTarget::Button(control) && self.control_enabled(control) {
            self.finish_control(control)
        } else {
            WindowAction::Redraw
        }
    }

    /// 【文字识别】【按钮完成】执行已完成的 control，返回动作；最后一项撤销后隐藏按钮并把焦点移向原文复制
    fn finish_control(&mut self, control: Control) -> WindowAction {
        let action = self.control(control);
        if control == Control::Undo && !self.can_undo() {
            self.focus_control(if self.control_enabled(Control::Copy) {
                FocusTarget::Button(Control::Copy)
            } else {
                FocusTarget::Window
            });
        }
        action
    }
}
