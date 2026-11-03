//! 【文件选择】【输入】键盘目录导航、文件名与默认取消的覆盖确认

use super::{
    layout::{Control, contains},
    save::{self, ExistingFile},
    state::{Field, Picker},
};
use crate::ui::fonts::UiFonts;
use crate::{
    backend::wayland::window::{KeyInput, LEFT_BUTTON, WindowEvent},
    keys::Key,
};
use std::{io, path::PathBuf};

pub enum Action {
    None,
    Redraw,
    Cancel,
    List {
        path: PathBuf,
        hidden: bool,
    },
    MakeFolder(PathBuf),
    Save {
        path: PathBuf,
        expected: Option<ExistingFile>,
    },
}

impl Picker {
    /// 【文件选择】【目录请求】请求读取 path；返回后台列表动作，保留当前列表直到读取成功
    fn navigate(&mut self, path: PathBuf) -> Action {
        self.pending = Some(path.clone());
        self.status = Some("Loading folder...".into());
        self.dragging = false;
        Action::List {
            path,
            hidden: self.hidden,
        }
    }

    /// 【文件选择】【保存请求】校验文件名和已有目标，必要时打开覆盖确认；返回保存或重绘动作
    fn save_request(&mut self) -> Action {
        if self.pending.is_some() {
            self.status = Some("Wait for the folder to load".into());
            return Action::Redraw;
        }
        if self.path.text() != self.directory.to_string_lossy() {
            self.focus = Some(Field::Path);
            self.status = Some("Press Enter to open the edited folder path".into());
            return Action::Redraw;
        }
        let result = if self.creating_folder {
            let name = self.filename.text();
            if name.is_empty() || name.contains(['/', '\\']) || matches!(name.as_str(), "." | "..")
            {
                Err(io::Error::other("enter a plain folder name"))
            } else {
                return Action::MakeFolder(self.directory.join(name));
            }
        } else {
            self.target()
                .and_then(|path| save::existing(&path).map(|expected| (path, expected)))
        };
        match result {
            Ok((path, None)) => Action::Save {
                path,
                expected: None,
            },
            Ok((path, Some(expected))) => {
                self.overwrite = Some((path, expected));
                self.confirm_replace = false;
                Action::Redraw
            }
            Err(error) => {
                self.status = Some(error.to_string());
                Action::Redraw
            }
        }
    }

    /// 【文件选择】【按钮动作】执行 control，fonts 用于文件名重置；返回后台请求或界面动作
    fn control(&mut self, control: Control, fonts: &mut UiFonts) -> Action {
        match control {
            Control::Cancel if self.creating_folder => {
                self.creating_folder = false;
                self.filename.set(
                    &self.previous_filename.take().unwrap_or_default(),
                    fonts,
                    true,
                );
                Action::Redraw
            }
            Control::Cancel => Action::Cancel,
            Control::Save => self.save_request(),
            Control::Home => {
                self.navigate(dirs::home_dir().unwrap_or_else(|| self.directory.clone()))
            }
            Control::Up => self.navigate(
                self.directory
                    .parent()
                    .unwrap_or(&self.directory)
                    .to_path_buf(),
            ),
            Control::Hidden => {
                self.hidden = !self.hidden;
                self.navigate(self.directory.clone())
            }
            Control::NewFolder => {
                if self.creating_folder {
                    return Action::None;
                }
                if self.pending.is_some() {
                    return Action::None;
                }
                self.creating_folder = true;
                self.previous_filename = Some(self.filename.text());
                self.filename.set("", fonts, false);
                self.focus = Some(Field::Filename);
                self.status = Some("Enter a new folder name".into());
                Action::Redraw
            }
            Control::ConfirmCancel => {
                self.overwrite = None;
                Action::Redraw
            }
            Control::ConfirmReplace => {
                self.overwrite
                    .take()
                    .map_or(Action::Redraw, |(path, expected)| Action::Save {
                        path,
                        expected: Some(expected),
                    })
            }
        }
    }

    /// 【文件选择】【按键处理】处理 input；覆盖确认默认取消，Enter 不会意外覆盖已有文件
    fn key(&mut self, input: &KeyInput, fonts: &mut UiFonts) -> Action {
        if self.overwrite.is_some() {
            return match input.key {
                Some(Key::Escape) => self.control(Control::ConfirmCancel, fonts),
                Some(Key::Tab | Key::Left | Key::Right) => {
                    self.confirm_replace = !self.confirm_replace;
                    Action::Redraw
                }
                Some(Key::Enter) => self.control(
                    if self.confirm_replace {
                        Control::ConfirmReplace
                    } else {
                        Control::ConfirmCancel
                    },
                    fonts,
                ),
                _ => Action::None,
            };
        }
        if input.key == Some(Key::Escape) {
            return self.control(Control::Cancel, fonts);
        }
        if input.mods.ctrl && input.key == Some(Key::Char('l')) {
            self.focus = Some(Field::Path);
            self.path.select_all(fonts);
            return Action::Redraw;
        }
        if input.mods.alt && input.key == Some(Key::Up) {
            return self.control(Control::Up, fonts);
        }
        if input.key == Some(Key::Tab) {
            self.focus = Some(if self.focus == Some(Field::Filename) {
                Field::Path
            } else {
                Field::Filename
            });
            return Action::Redraw;
        }
        if input.key == Some(Key::Enter) {
            return if self.focus == Some(Field::Path) {
                let path = self.path.text();
                let path = if path == "~" {
                    dirs::home_dir().unwrap_or_default()
                } else if let Some(suffix) = path.strip_prefix("~/") {
                    dirs::home_dir().unwrap_or_default().join(suffix)
                } else {
                    PathBuf::from(path)
                };
                self.navigate(if path.is_absolute() {
                    path
                } else {
                    self.directory.join(path)
                })
            } else if self.focus.is_none()
                && self
                    .selected
                    .is_some_and(|index| self.listing.get(index).is_some_and(|item| item.directory))
            {
                self.navigate(self.listing[self.selected.unwrap()].path.clone())
            } else {
                self.save_request()
            };
        }
        if let Some(field) = self.focus {
            if self.field(field).0.key(fonts, input) {
                return Action::Redraw;
            }
            return Action::None;
        }
        let old = self.selected.unwrap_or(0);
        self.selected = match input.key {
            Some(Key::Up) => Some(old.saturating_sub(1)),
            Some(Key::Down) => Some((old + 1).min(self.listing.len().saturating_sub(1))),
            _ => return Action::None,
        }
        .filter(|_| !self.listing.is_empty());
        let rows = self.clamp_scroll();
        if let Some(selected) = self.selected {
            self.scroll = self
                .scroll
                .min(selected)
                .max(selected.saturating_sub(rows - 1));
            if !self.listing[selected].directory {
                self.filename
                    .set(&self.listing[selected].name.to_string_lossy(), fonts, true);
            }
        }
        Action::Redraw
    }

    /// 【文件选择】【事件分派】处理 event 并用 fonts 排版；返回导航、保存、取消或重绘动作
    pub fn event(&mut self, event: &WindowEvent, fonts: &mut UiFonts) -> Action {
        match event {
            WindowEvent::Key(input) => self.key(input, fonts),
            WindowEvent::PointerButton {
                position,
                button: LEFT_BUTTON,
                pressed: true,
            } => {
                if let Some(control) = self.layout.control(*position, self.overwrite.is_some()) {
                    return self.control(control, fonts);
                }
                if self.overwrite.is_some() {
                    return Action::None;
                }
                for field in [Field::Path, Field::Filename] {
                    let (_, rect) = self.field(field);
                    if contains(rect, *position) {
                        self.focus = Some(field);
                        self.dragging = true;
                        self.field(field).0.pointer(
                            fonts,
                            (position.0 - rect.x() as f64, position.1 - rect.y() as f64),
                            false,
                        );
                        return Action::Redraw;
                    }
                }
                if contains(self.layout.list, *position) {
                    self.focus = None;
                    let index =
                        self.scroll + ((position.1 - self.layout.list.y() as f64) / 28.0) as usize;
                    if let Some(item) = self.listing.get(index) {
                        if item.directory {
                            return self.navigate(item.path.clone());
                        }
                        self.selected = Some(index);
                        self.filename.set(&item.name.to_string_lossy(), fonts, true);
                    }
                    return Action::Redraw;
                }
                Action::None
            }
            WindowEvent::PointerButton { pressed: false, .. } | WindowEvent::PointerLeave => {
                self.dragging = false;
                Action::None
            }
            WindowEvent::PointerMove { position } if self.dragging => {
                if let Some(field) = self.focus {
                    let (editor, rect) = self.field(field);
                    editor.pointer(
                        fonts,
                        (position.0 - rect.x() as f64, position.1 - rect.y() as f64),
                        true,
                    );
                }
                Action::Redraw
            }
            WindowEvent::Scroll { position, delta }
                if self.overwrite.is_none() && contains(self.layout.list, *position) =>
            {
                let step = (delta.1 / 28.0).round() as isize;
                let step = if step == 0 && delta.1 != 0.0 {
                    delta.1.signum() as isize
                } else {
                    step
                };
                self.scroll = (self.scroll as isize + step).max(0) as usize;
                self.clamp_scroll();
                Action::Redraw
            }
            _ => Action::None,
        }
    }
}
