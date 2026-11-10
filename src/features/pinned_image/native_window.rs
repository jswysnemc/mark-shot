//! 【原生贴图】【共享内容】原图独立保存，显示缩放、旋转、菜单与输出使用原生窗口契约

use crate::{
    backend::window::{
        self, WindowAction, WindowContent, WindowCursor, WindowEvent, WindowPopupAction, WindowSpec,
    },
    features::pinned_image::{
        menu::{self, PinAction},
        menu_draw::{self, Row},
        model::PinnedImage,
    },
    types::popup::{PopupEvent, PopupFrame},
};
mod actions;
mod popup;
mod text;
mod translation;

use popup::Menu;
use tiny_skia::{Pixmap, PixmapPaint, Transform};

struct Pin {
    image: PinnedImage,
    position_offset: Option<(f64, f64)>,
    initial: Option<(i32, i32)>,
    menu: Option<Menu>,
    next_menu: u64,
    fonts: cosmic_text::FontSystem,
    cache: cosmic_text::SwashCache,
    icons: crate::renderer::Icons,
    wake: Option<window::WindowWake>,
    text: crate::ocr::OcrView,
    text_selection: bool,
    selecting: bool,
    ocr: Option<text::Job>,
    copy_after_ocr: bool,
    window_after_ocr: bool,
    translation: crate::features::pinned_image::translation::Translation,
    translation_task: Option<crate::translation::task::Task>,
    translation_retired: Vec<crate::translation::task::Task>,
    top_supported: bool,
    menu_anchor: (f32, f32),
    feedback: crate::features::pinned_image::feedback::Feedback,
    model_dialog: Option<window::WindowDialog>,
    model_waiting: bool,
}

impl WindowContent for Pin {
    /// 【原生贴图】【更新期限】无参数，仅在完成提示或退出任务待回收时设置期限，空闲窗口保持阻塞
    fn next_deadline(&self) -> Option<std::time::Instant> {
        [
            self.feedback.deadline(),
            (!self.translation_retired.is_empty())
                .then(|| std::time::Instant::now() + std::time::Duration::from_millis(10)),
        ]
        .into_iter()
        .flatten()
        .min()
    }
    /// 【原生贴图】【后台通知】按wake记录窗口唤醒句柄，不启动空闲轮询
    fn connected(&mut self, wake: window::WindowWake) {
        self.wake = Some(wake);
    }
    /// 【原生贴图】【置顶能力】按supported记录平台是否提供真实置顶
    fn stacking_supported(&mut self, supported: bool) {
        self.top_supported = supported;
    }
    /// 【原生贴图】【原始位置】无参数，返回选区交接位置，缺省交给平台居中
    fn initial_position(&self) -> Option<(i32, i32)> {
        self.initial
    }
    /// 【原生贴图】【预览尺寸】无参数，返回缩放后的逻辑尺寸，原生窗口跟随预览，原图字节保持独立
    fn preferred_size(&self) -> Option<(u32, u32)> {
        Some(self.image.size())
    }
    /// 【原生贴图】【锚点位移】取出本轮缩放的逻辑位移，消费后不在后续重绘重复移动
    fn take_position_offset(&mut self) -> Option<(f64, f64)> {
        self.position_offset.take()
    }
    /// 【原生贴图】【滚轮预览】按input纵轴缩放并保持原图锚点，返回是否重绘；菜单期间及纯横向输入不修改图像
    fn scroll(&mut self, input: window::ScrollInput) -> WindowAction {
        use crate::features::pinned_image::wheel;
        if self.menu.is_some() {
            return WindowAction::None;
        }
        let Some(factor) = wheel::factor(input.angle.1, -input.delta.1) else {
            return WindowAction::None;
        };
        let before = self.image.size();
        if !self.image.scale(self.image.zoom * factor) {
            return WindowAction::None;
        }
        let previous = self.position_offset.unwrap_or_default();
        // 1. 【原生贴图】【连续消息】绘制前可能收到多个滚轮事件，扣除待应用位移后再计算当前局部锚点
        let point = (input.position.0 - previous.0, input.position.1 - previous.1);
        let shift = wheel::offset(point, before, self.image.size());
        self.position_offset = Some((previous.0 + shift.0, previous.1 + shift.1));
        WindowAction::Redraw
    }
    /// 【原生贴图】【置顶状态】无参数，返回当前原生置顶请求
    fn always_on_top(&self) -> bool {
        self.image.always_on_top && !self.model_waiting
    }
    /// 【原生贴图】【识别引导】无参数，交付所属模态窗口，避免在菜单回调中阻塞贴图事件循环
    fn take_dialog(&mut self) -> Option<window::WindowDialog> {
        self.model_dialog.take()
    }
    /// 【原生贴图】【引导完成】按id与accepted恢复等待动作，取消时保留原图并解除翻译占用
    fn dialog_closed(&mut self, id: u64, accepted: bool) -> WindowAction {
        if id != 0x4f4352 || !self.model_waiting {
            return WindowAction::None;
        }
        self.model_waiting = false;
        if accepted {
            let result = self.start_ocr().map(|_| ());
            if result.is_err() {
                self.translation.after_ocr = false;
                self.copy_after_ocr = false;
                self.window_after_ocr = false;
            }
            self.report(result);
        } else {
            self.translation.after_ocr = false;
            self.copy_after_ocr = false;
            self.window_after_ocr = false;
            self.text_selection = false;
            self.feedback.clear();
        }
        WindowAction::Redraw
    }
    /// 【原生贴图】【原图绘制】按canvas和scale缩放显示原图，保留原始像素用于复制与保存
    fn draw(&mut self, canvas: &mut Pixmap, scale: f32) {
        canvas.draw_pixmap(
            0,
            0,
            self.image.image.as_ref(),
            &PixmapPaint {
                quality: tiny_skia::FilterQuality::Bilinear,
                ..Default::default()
            },
            Transform::from_scale(
                canvas.width() as f32 / self.image.image.width() as f32,
                canvas.height() as f32 / self.image.image.height() as f32,
            ),
            None,
        );
        self.draw_text(canvas);
        self.feedback
            .draw(canvas, scale, &mut self.fonts, &mut self.cache);
    }
    /// 【原生贴图】【内容输入】按event处理移动、菜单、缩放和快捷键，返回窗口动作，错误记录明确反馈
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        let action = match event {
            WindowEvent::Wake => {
                let expired = self.feedback.expire();
                return if self.poll_text() || expired {
                    WindowAction::Redraw
                } else {
                    WindowAction::None
                };
            }
            WindowEvent::PointerMove { position } if self.selecting => {
                self.text.extend_drag(self.image_point(position));
                return WindowAction::Redraw;
            }
            WindowEvent::PointerButton {
                button: window::LEFT_BUTTON,
                pressed: true,
                position,
            } => {
                if self.text_selection && self.text.line_at(self.image_point(position)).is_some() {
                    self.text.begin_drag(self.image_point(position));
                    self.selecting = true;
                    return WindowAction::Redraw;
                }
                return WindowAction::Move;
            }
            WindowEvent::PointerButton {
                button: window::LEFT_BUTTON,
                pressed: false,
                ..
            } => {
                self.selecting = false;
                None
            }
            WindowEvent::PointerButton {
                position,
                button: window::RIGHT_BUTTON,
                pressed: true,
            } => {
                self.menu_anchor = (position.0 as f32, position.1 as f32);
                self.open_menu(false);
                return WindowAction::Redraw;
            }
            WindowEvent::Key(input) => match input.key {
                Some(crate::keys::Key::Escape) => Some(PinAction::Close),
                Some(crate::keys::Key::Char('c')) if input.mods.ctrl => {
                    Some(if self.text.has_selection() {
                        PinAction::CopySelectedText
                    } else {
                        PinAction::Copy
                    })
                }
                Some(crate::keys::Key::Char('a')) if input.mods.ctrl && self.text_selection => {
                    self.text.select_all();
                    return WindowAction::Redraw;
                }
                Some(crate::keys::Key::Char('s')) if input.mods.ctrl => Some(PinAction::SaveAs),
                Some(crate::keys::Key::Char('e')) if input.mods.ctrl => Some(PinAction::Edit),
                Some(crate::keys::Key::Char('v')) if input.mods.ctrl => {
                    if let Err(error) = crate::utils::spawn_self(&["--pin-clipboard"], &[]) {
                        log::error!("【原生贴图】【剪贴板新窗口】{error}");
                    }
                    None
                }
                _ => None,
            },
            WindowEvent::Resize { size } => {
                self.image.set_size(size);
                None
            }
            _ => None,
        };
        match action.map(|action| self.action(action)) {
            Some(Ok(action)) => action,
            Some(Err(error)) => {
                self.report(Err(error));
                WindowAction::Redraw
            }
            None => WindowAction::None,
        }
    }
    /// 【原生贴图】【菜单身份】无参数，返回当前菜单编号
    fn popup_id(&self) -> Option<u64> {
        self.menu_id()
    }
    /// 【原生贴图】【菜单像素】按scale返回变化后的菜单像素
    fn popup_frame(&mut self, scale: f32) -> Option<PopupFrame<'_>> {
        self.menu_frame(scale)
    }
    /// 【原生贴图】【菜单输入】按id与event处理菜单事务，返回窗口菜单动作
    fn popup_event(&mut self, id: u64, event: PopupEvent) -> WindowPopupAction {
        self.menu_event(id, event)
    }
    /// 【原生贴图】【菜单关闭】按id与action交付动作，返回窗口动作
    fn popup_closed(&mut self, id: u64, action: Option<usize>) -> WindowAction {
        self.menu_closed(id, action)
    }
    /// 【原生贴图】【移动光标】按position返回原图移动指针，菜单由独立表面处理
    fn cursor(&self, position: (f64, f64)) -> WindowCursor {
        if self.text_selection && self.text.line_at(self.image_point(position)).is_some() {
            WindowCursor::Text
        } else {
            WindowCursor::Grab
        }
    }
}
/// 【原生贴图】【原图入口】按bytes与density启动独立原生贴图，返回窗口结果，不初始化截图后端
pub fn run(bytes: &[u8], density: f32) -> crate::AppResult {
    run_at(bytes, density, None)
}
/// 【原生贴图】【定位入口】按bytes、density和position恢复选区位置，返回独立原生窗口结果
pub(crate) fn run_at(bytes: &[u8], density: f32, position: Option<(i32, i32)>) -> crate::AppResult {
    run_at_seed(bytes, density, position, None)
}
/// 【原生贴图】【种子入口】按bytes、density、position与seed启动原生贴图，恢复独立原图、识别框及译文
pub(crate) fn run_at_seed(
    bytes: &[u8],
    density: f32,
    position: Option<(i32, i32)>,
    seed: Option<crate::features::image_translation::seed::Seed>,
) -> crate::AppResult {
    let mut image = PinnedImage::decode(bytes, density)?;
    image.always_on_top = crate::config::get().pinned.always_on_top;
    let spec = WindowSpec {
        title: "Pinned Mark Shot".into(),
        app_id: "mark-shot-pin".into(),
        size: image.size(),
        minimum: (24, 24),
    };
    let mut pin = Pin {
        image,
        position_offset: None,
        initial: position,
        menu: None,
        next_menu: 0,
        fonts: crate::fonts::bundled(),
        cache: cosmic_text::SwashCache::new(),
        icons: crate::app::load_icons_cache(),
        wake: None,
        text: crate::ocr::OcrView::default(),
        text_selection: false,
        selecting: false,
        ocr: None,
        copy_after_ocr: false,
        window_after_ocr: false,
        translation: crate::features::pinned_image::translation::Translation::new(
            crate::config::translation::current()
                .map(|config| config.target_language)
                .unwrap_or_else(|_| "Simplified Chinese".into()),
        ),
        translation_task: None,
        translation_retired: Vec::new(),
        top_supported: false,
        menu_anchor: (0.0, 0.0),
        feedback: Default::default(),
        model_dialog: None,
        model_waiting: false,
    };
    if let Some(seed) = seed {
        pin.apply_seed(seed)?;
    }
    window::run(spec, pin)
}
