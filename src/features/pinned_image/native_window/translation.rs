//! 【原生贴图】【翻译任务】识别后启动翻译，按任务身份接收结果，语言变更取消旧请求

use super::*;
use crate::translation::{request::Request, task::Task};

impl Pin {
    /// 【原生贴图】【译文交接】按seed恢复识别和译文，不启动识别或网络请求，返回元数据校验结果
    pub(super) fn apply_seed(
        &mut self,
        seed: crate::features::image_translation::seed::Seed,
    ) -> crate::AppResult {
        seed.validate()?;
        self.clear_text();
        self.translation.source = crate::ocr::OcrText {
            lines: crate::features::image_translation::selection::source_lines(&seed.source_tokens),
        };
        self.translation.blocks = seed.blocks;
        self.translation.active = seed.show_translation && !self.translation.blocks.is_empty();
        self.sync_translation_text();
        Ok(())
    }

    /// 【原生贴图】【翻译请求】无参数，复用缓存或识别后继续翻译，返回启动错误
    pub(super) fn request_translation(&mut self) -> crate::AppResult {
        if self.translation_task.is_some() {
            return Ok(());
        }
        if !self.translation.blocks.is_empty() {
            self.translation.active = true;
            self.sync_translation_text();
        } else if self.translation.source.is_empty() {
            self.translation.after_ocr = true;
            if !self.start_ocr()? {
                self.translation.after_ocr = false;
            }
        } else {
            self.start_translation()?;
        }
        Ok(())
    }

    /// 【原生贴图】【翻译开始】无参数，按当前目标语言启动单次任务，返回配置或启动错误
    pub(super) fn start_translation(&mut self) -> crate::AppResult {
        self.translation.after_ocr = false;
        if !self.translation.prepare() {
            return Err("No text to translate".into());
        }
        let mut config = crate::config::translation::current()?;
        config.target_language = self.translation.target.clone();
        let wake = self.wake.clone().ok_or("pin window is not connected")?;
        let request = Request::image(
            &self.translation.source_blocks,
            (self.image.image.width(), self.image.image.height()),
        );
        let task = Task::start_request(
            &config,
            request,
            &self.translation.target,
            &crate::config::translation::path()?,
            move || wake.notify(),
        )?;
        self.translation.pending = Some(task.id);
        self.translation_task = Some(task);
        self.text.deselect();
        self.selecting = false;
        Ok(())
    }

    /// 【原生贴图】【翻译完成】无参数，只接收当前请求并重排译文，返回是否改变内容
    pub(super) fn poll_translation(&mut self) -> bool {
        self.translation_retired.retain(|task| !task.is_finished());
        let Some(result) = self.translation_task.as_mut().and_then(Task::take_result) else {
            return false;
        };
        let task = self.translation_task.take().unwrap();
        let id = task.id;
        if !task.is_finished() {
            self.translation_retired.push(task);
        }
        if self.translation.pending != Some(id) {
            return false;
        }
        match result {
            Ok(output) => {
                let mut texts = vec![String::new(); self.translation.source_blocks.len()];
                for segment in output.segments {
                    if let Ok(index) = usize::try_from(segment.id)
                        && let Some(text) = texts.get_mut(index)
                    {
                        *text = segment.text;
                    }
                }
                let blocks = crate::features::image_translation::layout::translated(
                    &self.image.image,
                    &self.translation.source_blocks,
                    &texts,
                    &mut self.fonts,
                );
                if self.translation.complete(id, blocks) {
                    self.sync_translation_text();
                }
            }
            Err(error) => {
                self.translation.pending = None;
                self.report(Err(error.into()));
            }
        }
        true
    }

    /// 【原生贴图】【翻译取消】无参数，取消当前请求并保留退出线程回收责任
    pub(super) fn cancel_translation(&mut self) {
        if let Some(mut task) = self.translation_task.take() {
            task.cancel();
            self.translation_retired.push(task);
        }
        self.translation.pending = None;
        self.translation.after_ocr = false;
    }

    /// 【原生贴图】【文字同步】无参数，让文字选择对应当前原文或译文，切换显示不丢弃另一份内容
    pub(super) fn sync_translation_text(&mut self) {
        self.text.set_lines(if self.translation.active {
            crate::features::image_translation::selection::lines(&self.translation.blocks)
        } else {
            self.translation.source.lines.clone()
        });
        self.selecting = false;
    }

    /// 【原生贴图】【语言切换】按index保存目标语言，成功后取消旧任务并移除旧译文，返回错误
    pub(super) fn choose_language(&mut self, index: usize) -> crate::AppResult {
        let language = crate::translation::language::OPTIONS
            .get(index)
            .ok_or("invalid translation language")?;
        if *language != self.translation.target {
            let target = crate::config::translation::save_target(language)?;
            self.cancel_translation();
            self.translation.set_target(target);
            self.sync_translation_text();
        }
        Ok(())
    }
}

impl Drop for Pin {
    /// 【原生贴图】【窗口关闭】无参数，取消识别并结束翻译任务，避免后台结果写入已关闭内容
    fn drop(&mut self) {
        self.ocr = None;
        if let Some(mut task) = self.translation_task.take() {
            task.shutdown();
        }
        for mut task in self.translation_retired.drain(..) {
            task.shutdown();
        }
    }
}
