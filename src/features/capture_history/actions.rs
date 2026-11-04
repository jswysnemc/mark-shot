//! 【截图历史】【窗口动作】选择操作、保存快照和配置更改

use super::{jobs::Task, layout::Control, window::HistoryWindow};
use crate::{
    backend::wayland::window::WindowAction, config::history::Update, ui::file_picker::Action,
};

impl HistoryWindow {
    /// 【截图历史】【操作资格】返回 control 是否可用；图像操作等待真实预览，空历史不能请求清空
    pub fn enabled(&self, control: Control) -> bool {
        if control == Control::Close {
            return true;
        }
        if self.jobs.foreground() {
            return false;
        }
        match control {
            Control::Copy | Control::Edit | Control::Pin | Control::Save | Control::Delete => self
                .model
                .selected()
                .is_some_and(|entry| self.model.preview(&entry.name).is_some()),
            Control::Clear => !self.model.entries.is_empty(),
            _ => true,
        }
    }

    /// 【截图历史】【控件动作】执行 control；当前选择捕获为任务身份，清空必须先确认
    pub fn control(&mut self, control: Control) -> WindowAction {
        if self.clear_dialog.is_some() {
            return WindowAction::None;
        }
        self.splitter_drag = None;
        self.scrollbar_drag = None;
        self.pressed = None;
        self.keyboard_pressed = false;
        if !self.enabled(control) {
            return WindowAction::None;
        }
        let name = self.model.selected().map(|entry| entry.name.clone());
        let task = match control {
            Control::Close => return WindowAction::Close,
            Control::Enabled => Task::Settings(Update::Enabled(!self.settings.enabled)),
            Control::Copy => match name {
                Some(name) => Task::Copy(name),
                None => return WindowAction::None,
            },
            Control::Pin => match name {
                Some(name) => Task::Pin(name),
                None => return WindowAction::None,
            },
            Control::Edit => match name {
                Some(name) => Task::Edit(name),
                None => return WindowAction::None,
            },
            Control::Save => match name {
                Some(name) => Task::Snapshot(name),
                None => return WindowAction::None,
            },
            Control::Delete => match name {
                Some(name) => Task::Delete(name),
                None => return WindowAction::None,
            },
            Control::Clear => {
                self.clear_dialog = Some(crate::features::text::input_method::focus::identity());
                self.clear_dialog_pending = true;
                self.last_click = None;
                self.hovered = None;
                self.hovered_row = None;
                return WindowAction::Redraw;
            }
        };
        self.request(task);
        WindowAction::Redraw
    }

    /// 【截图历史】【文件选择结果】处理 action 的目录或保存请求；保存使用打开对话框时的快照，不重复记录
    pub fn picker_action(&mut self, action: Action) -> WindowAction {
        match action {
            Action::None => return WindowAction::None,
            Action::Redraw => {}
            Action::Cancel => {
                self.picker = None;
                self.save_snapshot = None;
                self.refresh();
            }
            Action::List { path, hidden } => {
                self.request(Task::Directory { path, hidden });
            }
            Action::MakeFolder(path) => {
                if self.request(Task::MakeFolder(path))
                    && let Some(picker) = &mut self.picker
                {
                    picker.busy = true;
                    picker.status = Some("Creating folder...".into());
                }
            }
            Action::Save { path, expected } => {
                if let Some(png) = self.save_snapshot.clone()
                    && self.request(Task::Save {
                        png,
                        path,
                        expected,
                    })
                    && let Some(picker) = &mut self.picker
                {
                    picker.busy = true;
                    picker.status = Some("Saving screenshot...".into());
                }
            }
        }
        WindowAction::Redraw
    }
}
