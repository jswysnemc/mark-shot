//! 【截图历史】【窗口动作】选择操作、保存快照和配置更改

use super::{
    jobs::Task,
    layout::Control,
    window::{Focus, HistoryWindow},
};
use crate::{
    backend::wayland::window::WindowAction, config::history::Update, ui::file_picker::Action,
};

impl HistoryWindow {
    /// 【截图历史】【数量提交】验证当前数量字段并请求保存；无效值恢复旧值，返回是否接受
    pub fn commit_limit(&mut self) -> bool {
        let limit = self
            .limit
            .text()
            .parse::<usize>()
            .ok()
            .filter(|limit| (1..=200).contains(limit));
        self.focus = Focus::List;
        match limit {
            Some(limit) if limit != self.settings.limit => {
                self.request(Task::Settings(Update::Limit(limit)))
            }
            Some(_) => true,
            None => {
                self.limit
                    .set(&self.settings.limit.to_string(), &mut self.fonts, false);
                self.status = Some("Count must be between 1 and 200".into());
                false
            }
        }
    }

    /// 【截图历史】【控件动作】执行 control；当前选择捕获为任务身份，清空必须先确认
    pub fn control(&mut self, control: Control) -> WindowAction {
        self.dragging = false;
        if self.jobs.foreground() && !matches!(control, Control::Close | Control::ConfirmCancel) {
            return WindowAction::None;
        }
        let name = self.model.selected().map(|entry| entry.name.clone());
        let task = match control {
            Control::Close => return WindowAction::Close,
            Control::Enabled => Task::Settings(Update::Enabled(!self.settings.enabled)),
            Control::Minus => {
                Task::Settings(Update::Limit(self.settings.limit.saturating_sub(1).max(1)))
            }
            Control::Plus => Task::Settings(Update::Limit((self.settings.limit + 1).min(200))),
            Control::Refresh => {
                self.refresh();
                return WindowAction::Redraw;
            }
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
                self.clear_confirmation = true;
                self.confirm_clear = false;
                return WindowAction::Redraw;
            }
            Control::ConfirmCancel => {
                self.clear_confirmation = false;
                return WindowAction::Redraw;
            }
            Control::ConfirmClear => {
                self.clear_confirmation = false;
                Task::Clear
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
