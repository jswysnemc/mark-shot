//! 【贴图】【按需识别】仅显式启用文字选择时加载模型，后台完成事件唤醒阻塞循环

use super::runtime::Pin;
use crate::{
    AppResult,
    ocr::{OcrImage, OcrText, models::OcrModels},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

impl Pin {
    /// 【贴图】【文字识别启动】识别当前原图；没有模型时打开下载引导，返回任务启动状态
    pub fn start_ocr(&mut self) -> AppResult<bool> {
        if self.ocr_cancel.is_some() {
            return Ok(true);
        }
        let models = OcrModels::load();
        let files = crate::ocr::engine_choice::source(&models);
        let Some(files) = files else {
            let Some(files) = self.model_guide()? else {
                return Ok(false);
            };
            return self.start_ocr_with(files);
        };
        self.start_ocr_with(files)
    }

    /// 【贴图】【识别模型】按 files 启动当前原图一次识别，返回启动状态，不切换显示或目标语言
    fn start_ocr_with(&mut self, files: crate::ocr::engine_choice::Source) -> AppResult<bool> {
        let image = OcrImage {
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
        self.ocr_cancel = Some(cancel.clone());
        let generation = self.generation;
        let sender = self.ocr_sender.clone();
        std::thread::spawn(move || {
            let result = (|| {
                if cancel.load(Ordering::Relaxed) {
                    return Err("recognition cancelled".to_owned());
                }
                crate::features::pinned_image::recognition::recognize(image, files, &cancel)
            })();
            if !cancel.load(Ordering::Relaxed) {
                let _ = sender.send((generation, result));
            }
        });
        Ok(true)
    }

    /// 【贴图】【识别结果接收】接收 generation 对应 result；旋转后旧结果不应用，失败写入日志和通知
    pub fn receive_ocr(&mut self, generation: u64, result: Result<OcrText, String>) {
        if generation != self.generation {
            return;
        }
        self.ocr_cancel = None;
        match result {
            Ok(text) => {
                self.translation.source = text;
                self.sync_translation_text();
                if std::mem::take(&mut self.copy_after_ocr) {
                    let text = self.text.line_text(false);
                    if !text.is_empty() {
                        let result = crate::utils::copy_to_clipboard(&text);
                        self.report(result);
                    }
                }
                if self.translation.after_ocr {
                    let result = self.start_translation();
                    self.report(result);
                }
                self.dirty = true;
            }
            Err(error) => {
                self.copy_after_ocr = false;
                self.translation.after_ocr = false;
                log::error!("【贴图】【文字识别】识别失败: {error}");
                crate::backend::notify::send_blocking(crate::backend::notify::Notice::TextFailed(
                    error,
                ));
            }
        }
    }

    /// 【贴图】【识别坐标失效】旋转或关闭文字选择时取消旧任务，清除识别坐标，不等待线程
    pub fn clear_ocr(&mut self) {
        self.copy_after_ocr = false;
        self.cancel_translation();
        self.translation.clear();
        self.generation = self.generation.wrapping_add(1);
        if let Some(cancel) = self.ocr_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.text.clear();
        self.selecting = false;
    }

    /// 【贴图】【文字坐标】将 position 的显示坐标转换为未缩放图像坐标
    pub fn image_point(&self, position: (f64, f64)) -> (f64, f64) {
        let position = self.local_point(position);
        (
            position.0 * self.image.image.width() as f64 / f64::from(self.size.0),
            position.1 * self.image.image.height() as f64 / f64::from(self.size.1),
        )
    }
}
