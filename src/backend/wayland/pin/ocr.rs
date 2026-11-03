//! 【贴图】【按需识别】仅显式启用文字选择时加载模型，后台完成事件唤醒阻塞循环

use super::runtime::Pin;
use crate::{
    AppResult,
    features::pinned_image::actions,
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
        let files = models.active().and_then(|index| models.files(index));
        let Some(files) = files else {
            actions::open(&self.image.png()?, true)?;
            return Ok(false);
        };
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
                self.text.set_lines(text.lines);
                self.dirty = true;
            }
            Err(error) => {
                log::error!("【贴图】【文字识别】识别失败: {error}");
                crate::backend::notify::send_blocking(crate::backend::notify::Notice::TextFailed(
                    error,
                ));
            }
        }
    }

    /// 【贴图】【识别坐标失效】旋转或关闭文字选择时取消旧任务，清除识别坐标，不等待线程
    pub fn clear_ocr(&mut self) {
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
