//! 【扫码窗口】【生命周期】识别在后台执行，空闲结果窗口不持续轮询或保留原图

mod draw;
mod input;
mod layout;

#[cfg(test)]
mod tests;

use crate::{
    backend::window::{self, WindowSpec},
    ui::{fonts::UiFonts, text_editor::TextEditor},
};
use std::sync::mpsc::{self, Receiver};

pub(super) struct ResultWindow {
    fonts: UiFonts,
    cache: cosmic_text::SwashCache,
    editor: TextEditor,
    bytes: Option<Vec<u8>>,
    result: Option<Receiver<Result<Vec<super::Code>, String>>>,
    size: (u32, u32),
    status: String,
    count: usize,
    dragging: bool,
    scroll_drag: Option<crate::ui::scrollbar::Drag>,
    pressed: Option<layout::Control>,
    ready: bool,
    above_pin: bool,
    chinese: bool,
}

impl ResultWindow {
    /// 【扫码窗口】【创建状态】按 bytes、ready 和 above_pin 创建轻量状态，返回尚未识别的内容及来源层级
    fn new(bytes: Vec<u8>, ready: bool, above_pin: bool) -> Self {
        let chinese =
            crate::ui::language::Language::current() == crate::ui::language::Language::Chinese;
        let mut fonts = UiFonts::with_system_family("Sans Serif");
        let editor = TextEditor::new("", &mut fonts);
        Self {
            fonts,
            editor,
            cache: cosmic_text::SwashCache::new(),
            bytes: Some(bytes),
            result: None,
            size: (560, 360),
            status: if chinese {
                "正在识别…"
            } else {
                "Scanning…"
            }
            .into(),
            count: 0,
            dragging: false,
            scroll_drag: None,
            pressed: None,
            ready,
            above_pin,
            chinese,
        }
    }

    /// 【扫码窗口】【后台任务】按 wake 启动单个识别线程，结果通过有界通道交付并唤醒窗口
    fn start(&mut self, wake: window::WindowWake) {
        let Some(bytes) = self.bytes.take() else {
            return;
        };
        let (sender, receiver) = mpsc::sync_channel(1);
        self.result = Some(receiver);
        let spawn = std::thread::Builder::new()
            .name("code-scan".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(|| super::decoder::decode(&bytes))
                    .unwrap_or_else(|_| Err("扫码解码器异常 / Barcode decoder failed".into()));
                drop(bytes);
                crate::utils::memory::release_unused();
                let _ = sender.send(result);
                wake.notify();
            });
        if let Err(error) = spawn {
            self.result = None;
            self.status = error.to_string();
        }
    }

    /// 【扫码窗口】【结果接收】无参数，消费后台结果并更新文字内容，解码错误与未发现条码分别显示
    fn poll(&mut self) {
        let Some(receiver) = &self.result else { return };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(error) => Err(error.to_string()),
        };
        self.result = None;
        match result {
            Ok(codes) => {
                self.count = codes.len();
                self.status = if self.count == 0 {
                    if self.chinese {
                        "未识别到二维码或条形码"
                    } else {
                        "No QR code or barcode found"
                    }
                    .into()
                } else if self.chinese {
                    format!("识别到 {} 个码", self.count)
                } else {
                    format!("{} code(s) found", self.count)
                };
                let text = super::text(&codes);
                self.fonts.ensure_text(&text);
                self.editor = TextEditor::new(&text, &mut self.fonts);
            }
            Err(error) => {
                log::error!("【扫码】【图像识别】{error}");
                self.status = if self.chinese {
                    "识别失败"
                } else {
                    "Scan failed"
                }
                .into();
                self.fonts.ensure_text(&error);
                self.editor = TextEditor::new(&error, &mut self.fonts);
            }
        }
    }
}

/// 【扫码窗口】【原生入口】按 bytes、ready 和 above_pin 运行结果窗口，返回平台窗口或输入错误
pub(super) fn run(bytes: Vec<u8>, ready: bool, above_pin: bool) -> crate::AppResult {
    let state = ResultWindow::new(bytes, ready, above_pin);
    window::run(
        WindowSpec {
            title: "Mark Shot — Code Scan".into(),
            app_id: "mark-shot-code-scan".into(),
            size: state.size,
            minimum: (360, 240),
        },
        state,
    )
}
