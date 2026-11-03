//! 【截图历史】【窗口生命周期】独立原生窗口、异步结果和资源回收

use super::{
    Store,
    jobs::{Jobs, Response, Task},
    layout::{Control, Layout, contains},
    model::Model,
    splitter::Drag,
};
use crate::{
    AppResult,
    backend::wayland::window::{
        self, WindowAction, WindowContent, WindowCursor, WindowEvent, WindowSpec, WindowWake,
    },
    config::history::History,
    ui::file_picker::{self, Picker},
};
use cosmic_text::SwashCache;
use std::{io, sync::Arc};
use tiny_skia::Pixmap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Focus {
    #[default]
    List,
    Control(Control),
}

pub(super) struct HistoryWindow {
    pub fonts: crate::ui::fonts::UiFonts,
    pub cache: SwashCache,
    pub model: Model,
    pub settings: History,
    pub store: Store,
    pub jobs: Jobs,
    pub icons: crate::renderer::Icons,
    pub layout: Layout,
    pub focus: Focus,
    pub focused: bool,
    pub hovered: Option<Control>,
    pub status: Option<String>,
    pub clear_confirmation: bool,
    pub confirm_clear: bool,
    pub picker: Option<Picker>,
    pub save_snapshot: Option<Arc<Vec<u8>>>,
    pub left_width: f32,
    pub splitter_drag: Option<Drag>,
    pub scrollbar_drag: Option<super::scrollbar::Drag>,
    pub splitter_hovered: bool,
    pub hovered_row: Option<usize>,
    pub pressed: Option<Control>,
    pub keyboard_pressed: bool,
    pub last_click: Option<(std::time::Instant, String, (f64, f64))>,
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
        let fonts = crate::ui::fonts::UiFonts::with_system_family("Sans Serif");
        Ok(Self {
            fonts,
            cache: SwashCache::new(),
            model: Model::default(),
            settings,
            store,
            jobs: Jobs::default(),
            icons: super::chrome::icons(),
            layout: Layout::new((800, 520)),
            focus: Focus::Control(Control::Close),
            focused: true,
            hovered: None,
            status: None,
            clear_confirmation: false,
            confirm_clear: false,
            picker: None,
            save_snapshot: None,
            left_width: 210.0,
            splitter_drag: None,
            scrollbar_drag: None,
            splitter_hovered: false,
            hovered_row: None,
            pressed: None,
            keyboard_pressed: false,
            last_click: None,
            ready,
        })
    }

    /// 【截图历史】【水平重排】按 size、当前左栏与状态文字重建布局，保留实际分隔宽度，无返回值
    pub fn relayout(&mut self, size: (u32, u32)) {
        let height = super::chrome::status_buffer(
            &mut self.fonts,
            self.status.as_deref(),
            size.0 as f32 - 24.0,
        )
        .map_or(0.0, |buffer| buffer.layout_runs().count() as f32 * 19.0);
        self.layout = Layout::with_options(size, self.left_width, height);
        self.model.scroll = self
            .model
            .scroll
            .min(self.model.visible.len().saturating_sub(self.layout.rows()));
    }

    /// 【截图历史】【滚动查询】返回当前列表的滚动条几何，绘制和输入共用同一范围，无参数
    pub fn scrollbar(&self) -> Option<super::scrollbar::Geometry> {
        super::scrollbar::geometry(
            self.layout.list,
            self.model.visible.len(),
            self.layout.rows(),
            self.model.scroll,
        )
    }

    /// 【截图历史】【字体补充】text 包含非 ASCII 字符时加载系统字体；只执行一次，不初始化识别模型
    pub fn ensure_fonts(&mut self, text: &str) {
        self.fonts.ensure_text(text);
    }

    /// 【截图历史】【后台提交】把 task 放入当前存储任务队列；返回是否接受，不阻塞界面等待结果
    pub fn request(&mut self, task: Task) -> bool {
        self.jobs.request(self.store.clone(), task)
    }

    /// 【截图历史】【列表刷新】读取最新设置与目录后请求列表，不创建原版不存在的数量编辑状态，无参数、无返回值
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
                .take((self.layout.list.height() / super::layout::ROW).ceil() as usize)
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
                    self.model.refresh(entries);
                    self.model.scroll = self
                        .model
                        .scroll
                        .min(self.model.visible.len().saturating_sub(self.layout.rows()));
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
        self.relayout((self.layout.size.0 as u32, self.layout.size.1 as u32));
        super::draw::draw(self, canvas, scale);
    }
    /// 【截图历史】【窗口事件】处理 event；返回重绘、关闭或窗口管理动作
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        match event {
            WindowEvent::Wake => self.completed(),
            WindowEvent::Resize { size } => {
                self.relayout(size);
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
                self.splitter_drag = None;
                self.scrollbar_drag = None;
                self.pressed = None;
                self.keyboard_pressed = false;
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
        } else if self.splitter_drag.is_some() {
            WindowCursor::HorizontalResize
        } else if self.scrollbar_drag.is_some() {
            WindowCursor::Default
        } else if self.layout.resize_at(position) {
            WindowCursor::Resize
        } else if contains(self.layout.splitter, position) {
            WindowCursor::HorizontalResize
        } else if self
            .layout
            .control(position, false)
            .is_some_and(|control| control != Control::Enabled)
        {
            WindowCursor::Pointer
        } else {
            WindowCursor::Default
        }
    }
}

/// 【截图历史】【窗口运行】运行普通 Wayland 历史窗口，ready 决定是否向父进程确认首帧；关闭时释放全部资源
pub fn run(ready: bool) -> AppResult {
    window::run(
        WindowSpec {
            title: "Screenshot History".into(),
            app_id: "mark-shot-history".into(),
            size: (800, 520),
            minimum: (440, 320),
        },
        HistoryWindow::new(ready)?,
    )
}

#[cfg(test)]
#[path = "window_tests.rs"]
mod tests;
