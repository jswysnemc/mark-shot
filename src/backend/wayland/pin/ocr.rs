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
        self.feedback.recognizing();
        self.dirty = true;
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
        self.feedback.clear();
        match result {
            Ok(text) => {
                self.translation.source = text;
                self.sync_translation_text();
                let copy_after_ocr = std::mem::take(&mut self.copy_after_ocr);
                let line_text = copy_after_ocr.then(|| self.text.line_text(false));
                let result = apply_ocr_feedback(
                    &mut self.feedback,
                    copy_after_ocr,
                    &mut self.text_selection,
                    self.translation.after_ocr,
                    !self.text.lines.is_empty(),
                    line_text.as_deref().unwrap_or(""),
                    crate::utils::copy_to_clipboard,
                );
                self.report(result);
                if self.translation.after_ocr {
                    let result = self.start_translation();
                    self.report(result);
                }
                self.dirty = true;
            }
            Err(error) => {
                self.copy_after_ocr = false;
                self.translation.after_ocr = false;
                self.report(Err(error.into()));
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

/// 【贴图】【识别反馈】按 copy_after_ocr、text_selection、translate_after_ocr 与识别文本更新 feedback，调用 copy 写入剪贴板，返回剪贴板结果
fn apply_ocr_feedback(
    feedback: &mut crate::features::pinned_image::feedback::Feedback,
    copy_after_ocr: bool,
    text_selection: &mut bool,
    translate_after_ocr: bool,
    has_lines: bool,
    line_text: &str,
    copy: impl FnOnce(&str) -> AppResult,
) -> AppResult {
    // 1. 优先处理识别后复制分支，空文本提示未识别到文字，复制成功展示已复制文字胶囊
    if copy_after_ocr {
        if line_text.is_empty() {
            feedback.no_text();
            return Ok(());
        }
        copy(line_text)?;
        feedback.copied_text();
        return Ok(());
    }
    // 2. 纯划词识别完成时，有文字展示可拖拽选词胶囊，无文字关闭划词并提示未识别到文字
    if *text_selection && !translate_after_ocr {
        if has_lines {
            feedback.text_selection(true);
        } else {
            *text_selection = false;
            feedback.no_text();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::pinned_image::feedback::Feedback;

    /// 【贴图测试】【识别复制反馈】验证识别后复制在有文本、空文本及写入失败时的提示状态
    #[test]
    fn copy_after_ocr_updates_feedback_for_text_empty_and_error() {
        let mut feedback = Feedback::default();
        let mut text_selection = false;
        let mut copied = String::new();
        apply_ocr_feedback(
            &mut feedback,
            true,
            &mut text_selection,
            false,
            true,
            "hello",
            |text| {
                copied = text.to_owned();
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(copied, "hello");
        assert!(feedback.hud_title().is_some());
        assert!(feedback.task_title().is_none());

        let mut empty_feedback = Feedback::default();
        apply_ocr_feedback(
            &mut empty_feedback,
            true,
            &mut text_selection,
            false,
            false,
            "",
            |_| unreachable!("empty text should not write to clipboard"),
        )
        .unwrap();
        assert!(empty_feedback.hud_title().is_none());
        assert!(empty_feedback.task_title().is_some());

        let mut failed_feedback = Feedback::default();
        let err = apply_ocr_feedback(
            &mut failed_feedback,
            true,
            &mut text_selection,
            false,
            true,
            "hello",
            |_| Err("clipboard unavailable".into()),
        );
        assert!(err.is_err());
        assert!(failed_feedback.hud_title().is_none());
    }

    /// 【贴图测试】【划词识别反馈】验证划词识别完成时有文字提示可拖拽选词、无文字关闭划词并提示空结果
    #[test]
    fn text_selection_ocr_updates_feedback_and_clears_when_empty() {
        let mut feedback = Feedback::default();
        let mut text_selection = true;
        apply_ocr_feedback(
            &mut feedback,
            false,
            &mut text_selection,
            false,
            true,
            "",
            |_| Ok(()),
        )
        .unwrap();
        assert!(text_selection);
        assert!(feedback.hud_title().is_some());
        assert!(feedback.task_title().is_none());

        let mut empty_feedback = Feedback::default();
        apply_ocr_feedback(
            &mut empty_feedback,
            false,
            &mut text_selection,
            false,
            false,
            "",
            |_| Ok(()),
        )
        .unwrap();
        assert!(!text_selection);
        assert!(empty_feedback.hud_title().is_none());
        assert!(empty_feedback.task_title().is_some());
    }
}
