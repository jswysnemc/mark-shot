//! 【贴图翻译】【任务接入】识别后继续整段翻译，事件源一次唤醒，旧任务不覆盖当前原图或目标

use super::runtime::Pin;
use crate::{
    AppResult,
    features::image_translation::{layout, selection},
    translation::{request::Request, task::Task},
};

impl Pin {
    /// 【贴图翻译】【交接接收】按 seed 恢复原文和译文覆盖层，已有识别框不启动 OCR，当前任务先解除身份
    pub fn apply_translation_seed(&mut self, seed: crate::features::image_translation::seed::Seed) {
        if seed.source_tokens.is_empty() {
            return;
        }
        self.clear_ocr();
        self.translation.source = crate::ocr::OcrText {
            lines: crate::features::image_translation::selection::source_lines(&seed.source_tokens),
        };
        self.translation.blocks = seed.blocks;
        self.translation.active = seed.show_translation && !self.translation.blocks.is_empty();
        self.sync_translation_text();
        self.dirty = true;
    }
    /// 【贴图翻译】【翻译请求】复用现有译文或识别原图后继续翻译，返回启动错误，首次模型引导保留原动作
    pub fn request_translation(&mut self) -> AppResult {
        if self.translation_task.is_some() {
            return Ok(());
        }
        if !self.translation.blocks.is_empty() {
            self.set_translation_active(true);
            return Ok(());
        }
        if self.translation.source.is_empty() {
            self.translation.after_ocr = true;
            let result = self.start_ocr();
            self.translation.after_ocr = matches!(result, Ok(true));
            result?;
            return Ok(());
        }
        self.start_translation()
    }

    /// 【贴图翻译】【任务开始】使用原文段落、当前目标及服务启动单次任务，无参数，返回启动错误，不改写原图
    pub fn start_translation(&mut self) -> AppResult {
        self.translation.after_ocr = false;
        if !self.translation.prepare() {
            return Err("No text to translate".into());
        }
        let mut config = crate::config::translation::current()?;
        config.target_language = self.translation.target.clone();
        let wake = self.translation_wake.clone();
        let request = Request::image(
            &self.translation.source_blocks,
            (self.image.image.width(), self.image.image.height()),
        );
        let task = Task::start_request(
            &config,
            request,
            &self.translation.target,
            &crate::config::translation::path()?,
            move || wake.ping(),
        )?;
        self.translation.pending = Some(task.id);
        self.translation_task = Some(task);
        self.feedback.translating(&self.translation.target);
        self.text.deselect();
        self.selecting = false;
        self.dirty = true;
        self.cursor(self.last_position);
        Ok(())
    }

    /// 【贴图翻译】【结果完成】读取当前任务一次结果并排回原图，失败保留当前显示，无参数，不使用任务完成推测线程已退出
    pub fn poll_translation(&mut self) {
        self.translation_retired.retain(|task| !task.is_finished());
        let Some(result) = self.translation_task.as_mut().and_then(Task::take_result) else {
            return;
        };
        let task = self.translation_task.take().unwrap();
        let id = task.id;
        if !task.is_finished() {
            self.translation_retired.push(task);
        }
        if self.translation.pending != Some(id) {
            return;
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
                let blocks = layout::translated(
                    &self.image.image,
                    &self.translation.source_blocks,
                    &texts,
                    &mut self.font_system,
                );
                if self.translation.complete(id, blocks) {
                    self.feedback.translated(&self.translation.target);
                    self.sync_translation_text();
                } else {
                    self.report(Err("No usable translation was returned".into()));
                }
            }
            Err(error) => {
                self.translation.pending = None;
                self.report(Err(error.into()));
            }
        }
        self.cursor(self.last_position);
        self.dirty = true;
    }

    /// 【贴图翻译】【主动取消】取消当前任务并保留实际退出线程的回收责任，无参数，不阻塞窗口网络等待
    pub fn cancel_translation(&mut self) {
        if let Some(mut task) = self.translation_task.take() {
            task.cancel();
            self.translation_retired.push(task);
        }
        self.translation.pending = None;
        self.translation.after_ocr = false;
        self.feedback.clear();
    }

    /// 【贴图翻译】【显示切换】按 active 在已有原文和译文间切换，清除当前文字选择，不重新识别原图
    pub fn set_translation_active(&mut self, active: bool) {
        if active && self.translation.blocks.is_empty() {
            return;
        }
        self.translation.active = active;
        self.sync_translation_text();
        self.cursor(self.last_position);
        self.dirty = true;
    }

    /// 【贴图翻译】【选择切换】将当前显示的原文或译文选择行交给文字选择组件，无参数，不丢弃未显示全文
    pub fn sync_translation_text(&mut self) {
        let lines = if self.translation.active {
            selection::lines(&self.translation.blocks)
        } else {
            self.translation.source.lines.clone()
        };
        self.text.set_lines(lines);
        self.selecting = false;
    }

    /// 【贴图翻译】【目标保存】按 index 保存预设目标，成功后解除旧任务和缓存，失败保留原目标及显示
    pub fn choose_language(&mut self, index: usize) -> AppResult {
        let target = crate::translation::language::OPTIONS
            .get(index)
            .ok_or("invalid translation language")?;
        if *target == self.translation.target {
            return Ok(());
        }
        let target = crate::config::translation::save_target(target)?;
        self.cancel_translation();
        self.translation.set_target(target);
        self.sync_translation_text();
        self.cursor(self.last_position);
        self.dirty = true;
        Ok(())
    }
}
