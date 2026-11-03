//! 【截图历史】【窗口生命周期】独立原生窗口、异步结果和资源回收

use super::{
    Store,
    jobs::{Jobs, Response, Task},
    layout::{Control, Layout, contains},
    model::Model,
};
use crate::{
    AppResult,
    backend::wayland::window::{
        self, WindowAction, WindowContent, WindowCursor, WindowEvent, WindowSpec, WindowWake,
    },
    config::history::History,
    ui::{
        file_picker::{self, Picker},
        single_line::SingleLine,
    },
};
use cosmic_text::SwashCache;
use std::{io, sync::Arc};
use tiny_skia::Pixmap;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Focus {
    Search,
    Limit,
    #[default]
    List,
}

pub(super) struct HistoryWindow {
    pub fonts: crate::ui::fonts::UiFonts,
    pub cache: SwashCache,
    pub model: Model,
    pub settings: History,
    pub store: Store,
    pub jobs: Jobs,
    pub search: SingleLine,
    pub limit: SingleLine,
    pub layout: Layout,
    pub focus: Focus,
    pub focused: bool,
    pub hovered: Option<Control>,
    pub status: Option<String>,
    pub clear_confirmation: bool,
    pub confirm_clear: bool,
    pub picker: Option<Picker>,
    pub save_snapshot: Option<Arc<Vec<u8>>>,
    pub dragging: bool,
    pub last_click: Option<(std::time::Instant, String)>,
    ready: bool,
}

impl HistoryWindow {
    /// 【截图历史】【窗口构造】用 ready 决定是否向父进程确认首帧；初始化轻量界面，不创建历史目录或 OCR
    fn new(ready: bool) -> io::Result<Self> {
        let settings = crate::config::history::current();
        Self::with_settings(settings, ready)
    }

    /// 【截图历史】【配置构造】用 settings 与 ready 创建独立窗口状态；不读取用户配置或创建历史目录
    fn with_settings(settings: History, ready: bool) -> io::Result<Self> {
        let store = Store::new(settings.path()?, settings.byte_limit());
        let mut fonts = crate::ui::fonts::UiFonts::new();
        let search = SingleLine::new("", &mut fonts, 4096);
        let limit = SingleLine::new(&settings.limit.to_string(), &mut fonts, 3);
        Ok(Self {
            fonts,
            cache: SwashCache::new(),
            model: Model::default(),
            settings,
            store,
            jobs: Jobs::default(),
            search,
            limit,
            layout: Layout::new((760, 460)),
            focus: Focus::List,
            focused: true,
            hovered: None,
            status: Some("Loading history...".into()),
            clear_confirmation: false,
            confirm_clear: false,
            picker: None,
            save_snapshot: None,
            dragging: false,
            last_click: None,
            ready,
        })
    }

    /// 【截图历史】【字体补充】text 包含非 ASCII 字符时加载系统字体；只执行一次，不初始化识别模型
    pub fn ensure_fonts(&mut self, text: &str) {
        self.fonts.ensure_text(text);
    }

    /// 【截图历史】【后台提交】把 task 放入当前存储任务队列；返回是否接受，不阻塞界面等待结果
    pub fn request(&mut self, task: Task) -> bool {
        self.jobs.request(self.store.clone(), task)
    }

    /// 【截图历史】【列表刷新】读取最新设置与目录后请求列表；外部配置变化不覆盖未确认数量输入
    pub fn refresh(&mut self) {
        let settings = crate::config::history::current();
        match settings.path() {
            Ok(path) => {
                if self.store.directory() != path {
                    self.model.refresh(Vec::new());
                    self.last_click = None;
                }
                self.store = Store::new(path, settings.byte_limit());
            }
            Err(error) => {
                self.status = Some(error.to_string());
                return;
            }
        }
        if self.focus != Focus::Limit {
            self.limit
                .set(&settings.limit.to_string(), &mut self.fonts, false);
        }
        self.settings = settings;
        self.request(Task::List);
    }

    /// 【截图历史】【预览安排】优先读取当前选择，再读取可见缩略图；已有缓存或失败项不重复读取
    pub fn previews(&mut self) {
        if self.picker.is_some() || self.jobs.foreground() {
            return;
        }
        let mut names = Vec::new();
        if let Some(name) = &self.model.selected {
            names.push((name.clone(), true));
        }
        names.extend(
            self.model
                .visible
                .iter()
                .skip(self.model.scroll)
                .take(self.layout.rows())
                .map(|index| (self.model.entries[*index].name.clone(), false)),
        );
        for (name, full) in names {
            if (if full {
                self.model.preview(&name).is_none()
            } else {
                self.model.thumbnail(&name).is_none()
            }) && !self.model.failed.contains(&name)
            {
                if !self.jobs.preview_pending(&name) {
                    self.request(Task::Preview(name));
                }
                break;
            }
        }
    }

    /// 【截图历史】【错误反馈】在窗口内显示 message 与 error，并使用统一业务日志；不发出原生交互
    pub fn error(&mut self, message: &str, error: impl std::fmt::Display) {
        log::error!("【截图历史】【窗口操作】{message}: {error}");
        self.status = Some(format!("{message}: {error}"));
    }

    /// 【截图历史】【任务完成】处理后台响应并安排必要预览；空闲时无持续计时器或重复重绘
    fn completed(&mut self) -> WindowAction {
        let mut changed = false;
        while let Some(response) = self.jobs.take() {
            changed = true;
            match response {
                Response::Listed { path, .. } if path != self.store.directory() => {}
                Response::Listed {
                    result: Ok(entries),
                    ..
                } => {
                    if entries.iter().any(|entry| !entry.search.is_ascii()) {
                        self.ensure_fonts("字");
                    }
                    self.model.refresh(entries);
                    self.model.scroll = self
                        .model
                        .scroll
                        .min(self.model.visible.len().saturating_sub(self.layout.rows()));
                    if self.status.as_deref() == Some("Loading history...") {
                        self.status = None;
                    }
                    log::info!(
                        "【截图历史】【列表刷新】可用记录: {}",
                        self.model.entries.len()
                    );
                }
                Response::Listed {
                    result: Err(error), ..
                } => self.error("Cannot read history", error),
                Response::Preview {
                    name,
                    result: Ok(image),
                } => self.model.cache(name, image),
                Response::Preview {
                    name,
                    result: Err(error),
                } => {
                    if error.kind() != io::ErrorKind::Interrupted {
                        self.model.failed.insert(name.clone());
                        if self.model.selected.as_deref() == Some(&name) {
                            self.error("Cannot preview screenshot", error);
                        }
                    }
                }
                Response::Snapshot {
                    name,
                    result: Ok(png),
                } => {
                    let directory = dirs::picture_dir()
                        .filter(|path| path.is_dir())
                        .or_else(dirs::home_dir)
                        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                    self.ensure_fonts(&directory.to_string_lossy());
                    self.picker = Some(Picker::new(
                        directory.clone(),
                        &name,
                        (self.layout.size.0 as u32, self.layout.size.1 as u32),
                        &mut self.fonts,
                    ));
                    self.save_snapshot = Some(png);
                    self.request(Task::Directory {
                        path: directory,
                        hidden: false,
                    });
                }
                Response::Snapshot {
                    result: Err(error), ..
                } => self.error("Cannot read screenshot", error),
                Response::Directory { path, result } => {
                    if let Some(picker) = &mut self.picker {
                        picker.apply(&path, result, &mut self.fonts);
                    }
                }
                Response::Updated(Ok(settings)) => {
                    let prune = settings.limit != self.settings.limit;
                    self.limit
                        .set(&settings.limit.to_string(), &mut self.fonts, false);
                    self.settings = settings;
                    if prune {
                        self.request(Task::Prune(self.settings.limit));
                    }
                    self.status = Some(
                        if self.settings.enabled {
                            "Recording enabled"
                        } else {
                            "Recording disabled"
                        }
                        .into(),
                    );
                    log::info!(
                        "【截图历史】【策略更新】启用: {}, 数量: {}",
                        self.settings.enabled,
                        self.settings.limit
                    );
                }
                Response::Updated(Err(error)) => {
                    self.limit
                        .set(&self.settings.limit.to_string(), &mut self.fonts, false);
                    self.error("Cannot update history settings", error);
                }
                Response::Done {
                    label,
                    result: Ok(()),
                } => {
                    log::info!("【截图历史】【窗口操作】已完成: {label}");
                    self.status = Some(format!("{label} completed"));
                    match label {
                        "Clear" | "Delete" | "Prune" => self.refresh(),
                        "Save" => {
                            self.picker = None;
                            self.save_snapshot = None;
                            self.refresh();
                        }
                        "New folder" => {
                            if let Some(picker) = &mut self.picker {
                                picker.busy = false;
                                picker.creating_folder = false;
                                picker.filename.set(
                                    &picker.previous_filename.take().unwrap_or_default(),
                                    &mut self.fonts,
                                    true,
                                );
                                let path = picker.directory.clone();
                                let hidden = picker.hidden;
                                picker.pending = Some(path.clone());
                                self.request(Task::Directory { path, hidden });
                            }
                        }
                        _ => {}
                    }
                }
                Response::Done {
                    label,
                    result: Err(error),
                } => {
                    if let Some(picker) = &mut self.picker {
                        picker.busy = false;
                        picker.status = Some(format!("{label} failed: {error}"));
                    }
                    self.error(&format!("{label} failed"), error);
                }
            }
        }
        self.previews();
        if changed {
            WindowAction::Redraw
        } else {
            WindowAction::None
        }
    }
}

impl WindowContent for HistoryWindow {
    /// 【截图历史】【窗口连接】接入 wake 后开始按需列表读取；普通截图不创建此状态
    fn connected(&mut self, wake: WindowWake) {
        self.jobs.connect(wake);
        self.refresh();
    }
    /// 【截图历史】【首帧提交】记录首帧并按 ready 向父进程确认；不等待全部缩略图读取
    fn mapped(&mut self) {
        log::info!("【截图历史】【首帧提交】首个窗口缓冲已提交");
        if self.ready {
            use std::io::Write;
            if let Err(error) = std::io::stdout().write_all(b"\n") {
                self.error("Cannot acknowledge history window", error);
            }
        }
    }
    /// 【截图历史】【窗口绘制】按 scale 将列表和预览绘制到 canvas；无后台轮询
    fn draw(&mut self, canvas: &mut Pixmap, scale: f32) {
        super::draw::draw(self, canvas, scale);
    }
    /// 【截图历史】【窗口事件】处理 event；返回重绘、关闭或窗口管理动作
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        match event {
            WindowEvent::Wake => self.completed(),
            WindowEvent::Resize { size } => {
                self.layout = Layout::new(size);
                self.model.scroll = self
                    .model
                    .scroll
                    .min(self.model.visible.len().saturating_sub(self.layout.rows()));
                if let Some(picker) = &mut self.picker {
                    picker.layout = file_picker::layout::Layout::new(size);
                }
                self.previews();
                WindowAction::Redraw
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                self.dragging = false;
                if focused && self.picker.is_none() {
                    self.refresh();
                }
                WindowAction::Redraw
            }
            event => super::input::event(self, event),
        }
    }
    /// 【截图历史】【鼠标形状】按 position 返回文本或窗口调整指针；弹层阻止底层窗口调整
    fn cursor(&self, position: (f64, f64)) -> WindowCursor {
        if let Some(picker) = &self.picker {
            if picker.overwrite.is_none()
                && (contains(picker.layout.path, position)
                    || contains(picker.layout.filename, position))
            {
                WindowCursor::Text
            } else {
                WindowCursor::Default
            }
        } else if self.clear_confirmation {
            WindowCursor::Default
        } else if self.layout.resize_at(position) {
            WindowCursor::Resize
        } else if contains(self.layout.search, position) || contains(self.layout.limit, position) {
            WindowCursor::Text
        } else {
            WindowCursor::Default
        }
    }
}

/// 【截图历史】【窗口运行】运行普通 Wayland 历史窗口，ready 决定是否向父进程确认首帧；关闭时释放全部资源
pub fn run(ready: bool) -> AppResult {
    window::run(
        WindowSpec {
            title: "Mark Shot / Screenshot History".into(),
            app_id: "mark-shot-history".into(),
            size: (760, 460),
            minimum: (360, 360),
        },
        HistoryWindow::new(ready)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        backend::wayland::window::{KeyInput, LEFT_BUTTON},
        keys::{Key, Mods},
    };
    /// 【截图历史测试】【窗口状态】创建使用私有目录的未连接窗口；返回窗口和保留目录生命周期的句柄
    fn state() -> (HistoryWindow, tempfile::TempDir) {
        let directory = tempfile::tempdir().unwrap();
        let settings = History {
            directory: directory.path().join("history").to_string_lossy().into(),
            ..History::default()
        };
        (
            HistoryWindow::with_settings(settings, false).unwrap(),
            directory,
        )
    }
    /// 【截图历史测试】【按键构造】返回 key 与 mods 对应的窗口事件，不发送真实桌面输入
    fn key(key: Key, mods: Mods) -> WindowEvent {
        WindowEvent::Key(KeyInput {
            key: Some(key),
            mods,
            text: String::new(),
        })
    }
    /// 【截图历史测试】【清空确认】默认 Enter 取消，确认弹层不允许点击底层复制按钮
    #[test]
    fn clear_confirmation_defaults_to_cancel_and_blocks_click_through() {
        let (mut state, _directory) = state();
        state.control(Control::Clear);
        assert!(state.clear_confirmation);
        state.event(WindowEvent::PointerButton {
            position: (30.0, 410.0),
            button: LEFT_BUTTON,
            pressed: true,
        });
        assert!(state.clear_confirmation);
        state.event(key(Key::Enter, Mods::default()));
        assert!(!state.clear_confirmation);
        assert!(!state.store.directory().exists());
    }
    /// 【截图历史测试】【字段优先】搜索 Delete 只编辑字段；Escape 清空搜索后才关闭窗口
    #[test]
    fn search_editing_has_priority_over_image_deletion() {
        let (mut state, _directory) = state();
        state.focus = Focus::Search;
        state.search.set("capture", &mut state.fonts, true);
        state.event(key(Key::Delete, Mods::default()));
        assert_eq!(state.search.text(), "");
        state.search.set("capture", &mut state.fonts, false);
        assert_eq!(
            state.event(key(Key::Escape, Mods::default())),
            WindowAction::Redraw
        );
        assert_eq!(state.search.text(), "");
        assert_eq!(
            state.event(key(Key::Escape, Mods::default())),
            WindowAction::Close
        );
        assert!(!state.store.directory().exists());
    }
}
