//! 【截图另存为】【原生窗口】复用目录导航、覆盖确认和紧凑文件选择器

use std::{
    path::Path,
    sync::{Arc, OnceLock},
};

use super::{
    Outcome, Snapshot,
    jobs::{Completion, Jobs, Response, Task},
};
use crate::{
    backend::window::{
        self, WindowAction, WindowContent, WindowCursor, WindowEvent, WindowSpec, WindowWake,
    },
    ui::{
        file_picker::{
            self, Action, Picker,
            layout::{Layout, contains},
            state::Field,
        },
        fonts::UiFonts,
    },
};

struct SaveWindow {
    picker: Picker,
    snapshot: Snapshot,
    fonts: UiFonts,
    cache: cosmic_text::SwashCache,
    jobs: Option<Jobs>,
    completion: Completion,
}

impl SaveWindow {
    /// 【截图另存为】【窗口状态】用 snapshot、initial_path 和 completion 初始化文件选择器；不加载 OCR 或读取目录
    fn new(snapshot: Snapshot, initial_path: &Path, completion: Completion) -> Self {
        let directory = initial_path
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();
        let name = initial_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let mut fonts = UiFonts::new();
        fonts.ensure_text(&directory.to_string_lossy());
        fonts.ensure_text(&name);
        let picker = Picker::new(directory, &name, (660, 440), &mut fonts);
        Self {
            picker,
            snapshot,
            fonts,
            cache: cosmic_text::SwashCache::new(),
            jobs: None,
            completion,
        }
    }

    /// 【截图另存为】【后台请求】提交 task；没有任务通知源时只显示错误，不启动孤立线程
    fn request(&mut self, task: Task) {
        if let Some(jobs) = &mut self.jobs {
            jobs.submit(task);
        } else {
            self.picker.busy = false;
            self.picker.pending = None;
            self.picker.status = Some("File tasks are unavailable".into());
        }
    }

    /// 【截图另存为】【完成接收】应用已完成的目录或保存响应；实际保存成功或失败均结束对话框，供截图界面处理
    fn ready(&mut self) -> WindowAction {
        let mut changed = false;
        while let Some(response) = self.jobs.as_mut().and_then(Jobs::take) {
            changed = true;
            match response {
                Response::Listed { requested, listing } => {
                    self.picker.apply(&requested, listing, &mut self.fonts)
                }
                Response::MadeFolder(Ok(())) => {
                    self.picker.busy = false;
                    self.picker.creating_folder = false;
                    self.picker.filename.set(
                        &self.picker.previous_filename.take().unwrap_or_default(),
                        &mut self.fonts,
                        true,
                    );
                    let path = self.picker.directory.clone();
                    self.picker.pending = Some(path.clone());
                    self.request(Task::List {
                        path,
                        hidden: self.picker.hidden,
                        create: false,
                    });
                }
                Response::MadeFolder(Err(error)) => {
                    self.picker.busy = false;
                    self.picker.status = Some(format!("Cannot create folder: {error}"));
                }
                Response::Saved(Ok(())) => return WindowAction::Close,
                Response::Saved(Err(error)) => {
                    log::error!("【截图】【另存为】保存失败: {error}");
                    return WindowAction::Close;
                }
                Response::Failed(error) => {
                    if self.completion.get().is_some() {
                        log::error!("【截图】【另存为】保存任务失败: {error}");
                        return WindowAction::Close;
                    }
                    self.picker.busy = false;
                    self.picker.pending = None;
                    self.picker.status = Some(format!("File task failed: {error}"));
                    log::error!("【截图】【另存为】文件任务失败: {error}");
                }
            }
        }
        if changed {
            WindowAction::Redraw
        } else {
            WindowAction::None
        }
    }
}

impl WindowContent for SaveWindow {
    /// 【截图另存为】【通知接入】安装 wake 并创建默认保存目录；目录操作只在临时线程中执行
    fn connected(&mut self, wake: WindowWake) {
        self.jobs = Some(Jobs::new(wake, self.completion.clone()));
        self.request(Task::List {
            path: self.picker.directory.clone(),
            hidden: false,
            create: true,
        });
    }

    /// 【截图另存为】【内容绘制】用 scale 绘制文件选择器到 canvas；不绘制或缩放截图原图
    fn draw(&mut self, canvas: &mut tiny_skia::Pixmap, scale: f32) {
        canvas.fill(crate::theme::color::panel().into());
        file_picker::draw::draw(
            &mut self.picker,
            &mut self.fonts,
            &mut self.cache,
            canvas,
            scale,
        );
    }

    /// 【截图另存为】【事件处理】处理 event 并调度文件操作；保存期间只处理唤醒和尺寸，不产生重复请求
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        match event {
            WindowEvent::Wake => return self.ready(),
            WindowEvent::Resize { size } => {
                self.picker.layout = Layout::new(size);
                return WindowAction::Redraw;
            }
            _ if self.picker.busy => return WindowAction::None,
            _ => {}
        }
        match self.picker.event(&event, &mut self.fonts) {
            Action::None => WindowAction::None,
            Action::Redraw => WindowAction::Redraw,
            Action::Cancel => WindowAction::Close,
            Action::List { path, hidden } => {
                self.request(Task::List {
                    path,
                    hidden,
                    create: false,
                });
                WindowAction::Redraw
            }
            Action::MakeFolder(path) => {
                self.picker.busy = true;
                self.request(Task::MakeFolder(path));
                WindowAction::Redraw
            }
            Action::Save { path, expected } => {
                self.picker.busy = true;
                self.picker.status = Some("Saving screenshot...".into());
                self.request(Task::Save {
                    path,
                    expected,
                    png: self.snapshot.png.clone(),
                });
                WindowAction::Redraw
            }
        }
    }

    /// 【截图另存为】【鼠标形状】根据 position 返回文本或普通指针；忙碌期间不提示可编辑
    fn cursor(&self, position: (f64, f64)) -> WindowCursor {
        if !self.picker.busy
            && self.picker.overwrite.is_none()
            && [Field::Path, Field::Filename].iter().any(|field| {
                contains(
                    match field {
                        Field::Path => self.picker.layout.path,
                        Field::Filename => self.picker.layout.filename,
                    },
                    position,
                )
            })
        {
            WindowCursor::Text
        } else {
            WindowCursor::Default
        }
    }
}

/// 【截图另存为】【窗口执行】运行 snapshot 和 initial_path 的独立原生对话框；返回实际文件结果，不把已完成保存误报为取消
pub(super) fn run(snapshot: Snapshot, initial_path: &Path) -> crate::AppResult<Outcome> {
    let completion = Arc::new(OnceLock::new());
    let content = SaveWindow::new(snapshot.clone(), initial_path, completion.clone());
    let result = window::run(
        WindowSpec {
            title: "Mark Shot / Save Screenshot".into(),
            app_id: "mark-shot-save-as".into(),
            size: (660, 440),
            minimum: (340, 300),
        },
        content,
    );
    match completion.get() {
        Some(Ok(path)) => {
            snapshot.remember();
            Ok(Outcome::Saved(path.clone()))
        }
        Some(Err(error)) => Ok(Outcome::Failed(error.clone())),
        None => result.map(|()| Outcome::Cancelled),
    }
}
