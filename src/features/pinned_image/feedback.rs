//! 【贴图反馈】【共用提示】识别与翻译状态只绘入窗口预览，不修改原图及复制内容

use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::{controls, language::Language},
};
use std::time::{Duration, Instant};
use tiny_skia::{Pixmap, Rect};

#[derive(Default)]
pub(crate) struct Feedback {
    message: Option<(String, String)>,
    deadline: Option<Instant>,
}

impl Feedback {
    /// 【贴图反馈】【开始识别】无参数，显示持续识别状态，直到结果或取消替换提示
    pub fn recognizing(&mut self) {
        self.show("正在识别图片文字", "Recognizing image text", "", None);
    }
    /// 【贴图反馈】【开始翻译】按target显示当前目标，网络等待期间保持可见
    pub fn translating(&mut self, target: &str) {
        self.show("正在翻译", "Translating", target, None);
    }
    /// 【贴图反馈】【翻译完成】按target显示完成反馈，三秒后恢复无遮挡预览
    pub fn translated(&mut self, target: &str) {
        self.show(
            "翻译完成",
            "Translation complete",
            target,
            Some(Duration::from_secs(3)),
        );
    }
    /// 【贴图反馈】【失败结果】按detail显示错误，保留到重试或取消，关闭系统通知时仍然可见
    pub fn error(&mut self, detail: &str) {
        self.show(
            "操作失败，可重试或检查设置",
            "Failed; retry or check settings",
            detail,
            None,
        );
    }
    /// 【贴图反馈】【提示清除】无参数，移除旧操作状态和计时器
    pub fn clear(&mut self) {
        self.message = None;
        self.deadline = None;
    }
    /// 【贴图反馈】【定时截止】无参数，返回下一次需要清除完成反馈的时间，不产生空闲轮询
    pub fn deadline(&self) -> Option<Instant> {
        self.deadline
    }
    /// 【贴图反馈】【到期清除】无参数，过期时清除并返回需要重绘，其他状态返回否
    pub fn expire(&mut self) -> bool {
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.clear();
            true
        } else {
            false
        }
    }
    /// 【贴图反馈】【文案选择】按zh、en、detail和duration保存当前界面语言及可选期限，无返回值
    fn show(&mut self, zh: &str, en: &str, detail: &str, duration: Option<Duration>) {
        let title = if Language::current() == Language::Chinese {
            zh
        } else {
            en
        };
        self.message = Some((title.into(), detail.replace(['\r', '\n', '\t'], " ")));
        self.deadline = duration.map(|duration| Instant::now() + duration);
    }
    /// 【贴图反馈】【紧凑绘制】按canvas、scale、fonts和cache绘制最多两行提示，自适应窗口宽度，不放大图像
    pub fn draw(
        &self,
        canvas: &mut Pixmap,
        scale: f32,
        fonts: &mut cosmic_text::FontSystem,
        cache: &mut cosmic_text::SwashCache,
    ) {
        let Some((title, detail)) = &self.message else {
            return;
        };
        let scale = scale.max(0.1);
        let width = canvas.width() as f32 / scale;
        let height = canvas.height() as f32 / scale;
        if width < 48.0 || height < 28.0 {
            return;
        }
        crate::fonts::ui_fallback::ensure_text(fonts, &format!("{title} {detail}"));
        let palette = Palette::current();
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(fonts, cache);
        let panel_height = (if detail.is_empty() {
            32.0_f32
        } else {
            54.0_f32
        })
        .min(height - 8.0);
        let bounds = Rect::from_xywh(
            4.0,
            height - panel_height - 4.0,
            (width - 8.0).min(460.0),
            panel_height,
        )
        .unwrap();
        controls::frame(&mut painter, bounds, palette.base, palette.border, 6.0);
        for (index, text) in [title.as_str(), detail.as_str()].into_iter().enumerate() {
            if text.is_empty() || index as f32 * 22.0 + 22.0 > panel_height {
                continue;
            }
            let label = controls::fit_label(&mut painter, text, 11.0, bounds.width() - 16.0);
            painter.text(
                Rect::from_xywh(
                    bounds.left() + 8.0,
                    bounds.top() + 5.0 + index as f32 * 22.0,
                    bounds.width() - 16.0,
                    22.0,
                )
                .unwrap(),
                &label,
                TextStyle::new(11.0, palette.text),
            );
        }
    }
}
