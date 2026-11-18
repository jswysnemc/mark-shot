//! 【识别模型】【引导窗口】下载期间按需更新进度，空闲时事件循环阻塞，选择结果保持所属动作

use crate::{
    backend::window::{self, WindowAction, WindowContent, WindowEvent, WindowSpec},
    keys::Key,
    ocr::{
        engine_choice::{self, Source},
        models::{MODELS, ModelEvent, ModelStatus, OcrModels},
    },
    renderer::Painter,
    ui::model_popover::{self, ModelPopover, ModelPopoverElement, ModelRow},
};
use std::{
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};
use tiny_skia::Pixmap;

#[path = "frame.rs"]
mod frame;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

struct Guide {
    models: OcrModels,
    panel: ModelPopover,
    fonts: cosmic_text::FontSystem,
    cache: cosmic_text::SwashCache,
    icons: crate::renderer::Icons,
    completion: Arc<OnceLock<Source>>,
    pressed: Option<ModelPopoverElement>,
    close_hovered: bool,
    close_pressed: bool,
    size: (u32, u32),
    deadline: Option<Instant>,
    auto_close: bool,
}

impl Guide {
    /// 【识别模型】【显示比例】返回当前窗口可容纳模型组件的比例，无参数，标题栏保持紧凑逻辑高度
    fn fit(&self) -> f32 {
        frame::layout(self.size, self.panel.size).fit
    }

    /// 【识别窗口】【内容定位】无参数，按当前窗口与内容尺寸同步缩放后的绘制和命中原点，无返回值
    fn place_panel(&mut self) {
        let layout = frame::layout(self.size, self.panel.size);
        self.panel.render_pos = (layout.origin.0 / layout.fit, layout.origin.1 / layout.fit);
        self.panel.position = self.panel.render_pos;
    }

    /// 【识别模型】【指针换算】按 position 返回组件未缩放坐标，保证合成器限制窗口尺寸后命中仍与显示一致
    fn panel_point(&self, position: (f64, f64)) -> (f64, f64) {
        let fit = f64::from(self.fit());
        (position.0 / fit, position.1 / fit)
    }
    /// 【识别模型】【界面状态】按 models、completion 与 auto_close 创建复用模型面板，返回静止状态，不自动下载
    fn new(models: OcrModels, completion: Arc<OnceLock<Source>>, auto_close: bool) -> Self {
        let mut panel = ModelPopover::new();
        panel.open = true;
        panel.opacity = 1.0;
        let size = frame::preferred(panel.size);
        let mut result = Self {
            models,
            panel,
            fonts: crate::fonts::bundled(),
            cache: cosmic_text::SwashCache::new(),
            icons: crate::app::load_icons_cache(),
            completion,
            pressed: None,
            close_hovered: false,
            close_pressed: false,
            size,
            deadline: None,
            auto_close,
        };
        result.sync();
        result.size = frame::preferred(result.panel.size);
        result.place_panel();
        result
    }

    /// 【识别模型】【模型状态】同步当前已安装状态及进度到 panel，无参数，不创建后台任务
    fn sync(&mut self) {
        let changed = model_popover::setup::sync(&mut self.panel, &self.models);
        if changed || self.panel.pixmap.is_none() {
            model_popover::setup::prepare_text(&mut self.fonts, &self.panel);
        }
        self.panel
            .sync_rows((0..MODELS.len()).map(|index| ModelRow {
                status: self.models.status(index),
                active: self.models.active() == Some(index),
                size: self.models.download_size(index),
                recommended: self.models.installed_count() == 0
                    && index == self.models.recommended(),
            }));
        self.place_panel();
        self.deadline = self
            .models
            .needs_poll()
            .then(|| Instant::now() + Duration::from_millis(50));
    }

    /// 【识别模型】【明确选择】按 element 下载、取消或选择已安装模型，返回是否关闭，选择完成才继续原动作
    fn activate(&mut self, element: ModelPopoverElement) -> WindowAction {
        if model_popover::setup::activate(&mut self.panel, &mut self.models, element) {
            self.sync();
            if !self.panel.setup.message.is_empty() {
                return WindowAction::Redraw;
            }
            return self.complete();
        }
        let index = match element {
            ModelPopoverElement::Row(index) | ModelPopoverElement::Button(index) => index,
            _ => return WindowAction::None,
        };
        match self.models.status(index) {
            ModelStatus::Installed => {
                self.models.set_active(Some(index));
                return self.complete();
            }
            ModelStatus::Queued | ModelStatus::Downloading(_) => self.models.cancel(index),
            ModelStatus::Missing | ModelStatus::Failed => self.models.download(index),
        }
        self.sync();
        WindowAction::Redraw
    }

    /// 【识别安装】【完整准备】无参数，仅在首次引导模式且所选引擎全部资源就绪时自动关闭，资源管理模式保持打开
    fn complete(&mut self) -> WindowAction {
        if self.auto_close
            && let Some(source) = engine_choice::source(&self.models)
        {
            let _ = self.completion.set(source);
            WindowAction::Close
        } else {
            self.sync();
            WindowAction::Redraw
        }
    }
}

impl Drop for Guide {
    /// 【识别模型】【窗口结束】停止当前下载队列，无参数，不在窗口关闭后继续排队任务
    fn drop(&mut self) {
        self.models.shutdown();
    }
}

impl WindowContent for Guide {
    /// 【识别窗口】【内容尺寸】无参数，返回展开或收起后完整内容需要的窗口大小
    fn preferred_size(&self) -> Option<(u32, u32)> {
        Some(frame::preferred(self.panel.size))
    }
    /// 【识别模型】【圆角外观】无参数，返回启用圆角外侧透明裁切
    fn rounded_corners(&self) -> bool {
        true
    }

    /// 【识别模型】【明确确认】无参数，仅在资源已就绪且用户完成选择后报告确认，关闭和取消返回否
    fn dialog_accepted(&self) -> bool {
        self.completion.get().is_some()
    }
    /// 【识别模型】【更新期限】返回下载进度下一次期限，无参数，静止引导无定时工作
    fn next_deadline(&self) -> Option<Instant> {
        self.deadline
    }

    /// 【识别模型】【面板绘制】按 scale 在 canvas 绘制模型组件和紧凑标题栏，缩小窗口时整体适配
    fn draw(&mut self, canvas: &mut Pixmap, scale: f32) {
        self.size = (
            (canvas.width() as f32 / scale).round() as u32,
            (canvas.height() as f32 / scale).round() as u32,
        );
        canvas.fill(crate::theme::window::Palette::current().window.color());
        let fit = self.fit();
        self.place_panel();
        {
            let mut painter = Painter::new(canvas)
                .with_scale(scale)
                .with_icons(&self.icons)
                .with_text(&mut self.fonts, &mut self.cache);
            frame::draw(
                &mut painter,
                self.size,
                self.close_hovered,
                self.close_pressed,
            );
            model_popover::draw_model_popover(
                &mut painter.with_scale(scale * fit),
                &mut self.panel,
            );
        }
        crate::ui::window_frame::draw(canvas, scale);
    }

    /// 【识别模型】【输入处理】处理 event 的下载、选择、取消与进度，右键不能开始下载，返回窗口动作
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        match event {
            WindowEvent::Wake => {
                for event in self.models.poll() {
                    match event {
                        ModelEvent::Ready(index) => {
                            self.models.set_active(Some(index));
                        }
                        ModelEvent::Failed(_, error) => {
                            log::error!("【识别模型】【引导下载】下载失败: {error}")
                        }
                        ModelEvent::Progress => {}
                    }
                }
                self.sync();
                self.complete()
            }
            WindowEvent::Resize { size } => {
                self.size = size;
                self.place_panel();
                WindowAction::Redraw
            }
            WindowEvent::Key(input) => match input.key {
                Some(Key::Escape) => WindowAction::Close,
                Some(Key::Enter | Key::Space) => self
                    .panel
                    .hovered
                    .map_or(WindowAction::None, |item| self.activate(item)),
                Some(Key::Up | Key::Down) => {
                    let items = model_popover::navigation_items(&self.panel);
                    let old = items
                        .iter()
                        .position(|item| Some(*item) == self.panel.hovered);
                    let index = if input.key == Some(Key::Down) {
                        old.map_or(0, |index| (index + 1) % items.len())
                    } else {
                        old.map_or(items.len() - 1, |index| {
                            (index + items.len() - 1) % items.len()
                        })
                    };
                    self.panel.hovered = Some(items[index]);
                    WindowAction::Redraw
                }
                _ => WindowAction::None,
            },
            WindowEvent::PointerMove { position } => {
                let close = frame::layout(self.size, self.panel.size).close;
                let hovered =
                    crate::ui::toolbar::contains(close, (position.0 as f32, position.1 as f32));
                let item = self.panel.element_at(self.panel_point(position));
                let changed = self.close_hovered != hovered || self.panel.hovered != item;
                self.close_hovered = hovered;
                self.panel.hovered = item;
                if changed {
                    WindowAction::Redraw
                } else {
                    WindowAction::None
                }
            }
            WindowEvent::PointerLeave | WindowEvent::Focused(false) => {
                self.close_hovered = false;
                self.close_pressed = false;
                self.panel.hovered = None;
                self.pressed = None;
                WindowAction::Redraw
            }
            WindowEvent::PointerButton {
                position,
                button: window::LEFT_BUTTON,
                pressed,
            } => {
                let close = frame::layout(self.size, self.panel.size).close;
                let over_close =
                    crate::ui::toolbar::contains(close, (position.0 as f32, position.1 as f32));
                if self.close_pressed && !pressed {
                    self.close_pressed = false;
                    return if over_close {
                        WindowAction::Close
                    } else {
                        WindowAction::Redraw
                    };
                }
                if position.1 < f64::from(frame::HEADER) {
                    if pressed {
                        return if over_close {
                            self.close_pressed = true;
                            WindowAction::Redraw
                        } else {
                            WindowAction::Move
                        };
                    }
                    return WindowAction::None;
                }
                let item = self.panel.element_at(self.panel_point(position));
                if pressed {
                    self.pressed = item;
                    WindowAction::None
                } else {
                    let armed = self.pressed.take();
                    item.filter(|item| Some(*item) == armed)
                        .map_or(WindowAction::None, |item| self.activate(item))
                }
            }
            _ => WindowAction::None,
        }
    }
}

/// 【识别模型】【原生执行】按 models 打开独立窗口，返回明确选择的模型文件，窗口管理器关闭视为取消
pub(super) fn run(models: OcrModels) -> crate::AppResult<Option<Source>> {
    let completion = Arc::new(OnceLock::new());
    let content = Guide::new(models, completion.clone(), true);
    let size = content.size;
    window::run(
        WindowSpec {
            title: "Mark Shot / OCR".into(),
            app_id: "mark-shot-ocr-model-guide".into(),
            size,
            minimum: size,
        },
        content,
    )?;
    Ok(completion.get().cloned())
}

/// 【识别模型】【设置入口】无参数，创建资源管理模态窗口，允许持续下载多门语言与切换当前识别语言
pub(crate) fn dialog() -> window::WindowDialog {
    let content = Guide::new(OcrModels::load(), Arc::new(OnceLock::new()), false);
    window::WindowDialog {
        id: 0x4f4352,
        spec: WindowSpec {
            title: "Mark Shot / OCR".into(),
            app_id: "mark-shot-ocr-resources".into(),
            size: content.size,
            minimum: (280, 160),
        },
        content: Box::new(content),
    }
}
