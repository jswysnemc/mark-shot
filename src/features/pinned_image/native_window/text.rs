//! 【原生贴图】【文字识别】显式启用后后台识别，通过窗口通知交付结果，原图变化时取消旧任务

use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

pub(super) struct Job {
    cancel: Arc<AtomicBool>,
    receiver: mpsc::Receiver<Result<crate::ocr::OcrText, String>>,
}

impl Drop for Job {
    /// 【原生贴图】【识别取消】无参数，通知后台任务退出，不阻塞窗口关闭
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Pin {
    /// 【原生贴图】【识别开始】无参数，识别当前原图或打开模型引导，返回是否开始有效任务
    pub(super) fn start_ocr(&mut self) -> crate::AppResult<bool> {
        if self.ocr.is_some() {
            return Ok(true);
        }
        let files = match crate::ocr::engine_choice::source(&crate::ocr::models::OcrModels::load())
        {
            Some(files) => files,
            None => match crate::features::ocr_model_guide::run()? {
                Some(files) => files,
                None => return Ok(false),
            },
        };
        let wake = self.wake.clone().ok_or("pin window is not connected")?;
        let image = crate::ocr::OcrImage {
            rgb: self
                .image
                .image
                .data()
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|pixel| {
                    let white = 255 - pixel[3];
                    pixel[..3]
                        .iter()
                        .map(move |channel| channel.saturating_add(white))
                })
                .collect(),
            width: self.image.image.width(),
            height: self.image.image.height(),
            origin: (0.0, 0.0),
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let cancelled = cancel.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result =
                crate::features::pinned_image::recognition::recognize(image, files, &cancelled);
            if !cancelled.load(Ordering::Relaxed) && sender.send(result).is_ok() {
                wake.notify();
            }
        });
        self.ocr = Some(Job { cancel, receiver });
        Ok(true)
    }

    /// 【原生贴图】【后台完成】无参数，接收当前图像识别和翻译结果，返回是否需要重绘
    pub(super) fn poll_text(&mut self) -> bool {
        let mut changed = self.poll_translation();
        let result = self
            .ocr
            .as_ref()
            .and_then(|job| job.receiver.try_recv().ok());
        if let Some(result) = result {
            self.ocr = None;
            changed = true;
            match result {
                Ok(text) => {
                    self.translation.source = text;
                    self.sync_translation_text();
                    if std::mem::take(&mut self.copy_after_ocr) {
                        self.report(crate::utils::copy_to_clipboard(&self.text.line_text(false)));
                    }
                    if self.translation.after_ocr {
                        let result = self.start_translation();
                        self.report(result);
                    }
                }
                Err(error) => {
                    self.copy_after_ocr = false;
                    self.translation.after_ocr = false;
                    self.report(Err(error.into()));
                }
            }
        }
        changed
    }

    /// 【原生贴图】【原图失效】无参数，取消旧任务并清除坐标，缩放不调用此方法
    pub(super) fn clear_text(&mut self) {
        self.ocr = None;
        self.copy_after_ocr = false;
        self.cancel_translation();
        self.translation.clear();
        self.text.clear();
        self.selecting = false;
    }

    /// 【原生贴图】【图像坐标】按position返回未缩放原图中的文字坐标
    pub(super) fn image_point(&self, position: (f64, f64)) -> (f64, f64) {
        let size = self.image.size();
        (
            position.0 * f64::from(self.image.image.width()) / f64::from(size.0),
            position.1 * f64::from(self.image.image.height()) / f64::from(size.1),
        )
    }

    /// 【原生贴图】【文字覆盖绘制】按canvas绘制译文与文字选区，原始图像保持不变
    pub(super) fn draw_text(&mut self, canvas: &mut Pixmap) {
        let ratio = (
            canvas.width() as f32 / self.image.image.width() as f32,
            canvas.height() as f32 / self.image.image.height() as f32,
        );
        if self.translation.active {
            crate::features::image_translation::paint::viewport_xy(
                canvas,
                &self.translation.blocks,
                (0.0, 0.0),
                ratio,
                &mut self.fonts,
                &mut self.cache,
            );
        }
        if self.text_selection {
            let mut painter = crate::renderer::Painter::new(canvas);
            for index in 0..self.text.lines.len() {
                if let Some(selection) = self.text.line_selection(index)
                    && let Some(rect) = tiny_skia::Rect::from_ltrb(
                        selection.x.0 * ratio.0,
                        selection.y.0 * ratio.1,
                        selection.x.1 * ratio.0,
                        selection.y.1 * ratio.1,
                    )
                {
                    painter.fill_rounded(rect, 0.0, crate::theme::Rgba(72, 132, 245, 96));
                }
            }
        }
    }

    /// 【原生贴图】【失败反馈】按result记录失败并使用统一通知，成功不产生消息
    pub(super) fn report(&self, result: crate::AppResult) {
        if let Err(error) = result {
            log::error!("【原生贴图】【业务动作】{error}");
            crate::backend::notify::send_blocking(crate::backend::notify::Notice::Failed(
                error.to_string(),
            ));
        }
    }
}
