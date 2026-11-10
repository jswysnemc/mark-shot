//! 【翻译测试】【预览绘制】预制原文与译文独立展示，长错误可滚动阅读并复制完整内容

use super::super::layout::rect;
use super::{Preview, ResultState};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::{controls, fonts::UiFonts, scrollbar},
};
use cosmic_text::{Attrs, Buffer, Metrics, Shaping, Wrap};
use tiny_skia::Rect;

pub(in crate::features::preferences::translation) const HEIGHT: f32 = 278.0;

pub(in crate::features::preferences::translation) struct Layout {
    pub bounds: Rect,
    pub test: Rect,
    pub copy: Rect,
    pub source: Rect,
    pub result: Rect,
}

impl Layout {
    /// 【翻译测试】【区域布局】按 bounds 生成紧凑原文和结果区域，按钮与标题互不覆盖
    pub fn new(bounds: Rect) -> Self {
        Self {
            bounds,
            test: rect(bounds.right() - 110.0, bounds.top() + 4.0, 96.0, 26.0),
            copy: rect(bounds.right() - 210.0, bounds.top() + 4.0, 92.0, 26.0),
            source: rect(
                bounds.left() + 14.0,
                bounds.top() + 80.0,
                bounds.width() - 28.0,
                52.0,
            ),
            result: rect(
                bounds.left() + 14.0,
                bounds.top() + 164.0,
                bounds.width() - 28.0,
                100.0,
            ),
        }
    }

    /// 【翻译测试】【结果滚动条】按 preview 返回结果区域的溢出滚动几何，短结果不显示轨道
    pub fn scrollbar(&self, preview: &Preview) -> Option<scrollbar::Geometry> {
        scrollbar::geometry(
            self.result,
            preview.text_height,
            self.result.height() as usize - 12,
            preview.scroll,
        )
    }
}

pub(in crate::features::preferences::translation) struct Text {
    source: Buffer,
    result: Buffer,
}

/// 【翻译测试】【文本换行】按 text、width 和 fonts 返回按字符兜底换行的字形，长网址也不溢出边界
fn shape(text: &str, width: f32, fonts: &mut UiFonts) -> Buffer {
    fonts.ensure_text(text);
    let mut buffer = Buffer::new(fonts, Metrics::new(12.0, 18.0));
    buffer.set_size(Some(width.max(1.0)), None);
    buffer.set_wrap(Wrap::WordOrGlyph);
    buffer.set_text(
        text,
        &Attrs::new().family(cosmic_text::Family::SansSerif),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(fonts, false);
    buffer
}

/// 【翻译测试】【显示准备】按 preview、layout、chinese、fonts 和 source 排版正文并同步滚动上限，返回本帧字形
pub(in crate::features::preferences::translation) fn prepare(
    preview: &mut Preview,
    layout: &Layout,
    chinese: bool,
    fonts: &mut UiFonts,
    source: &str,
) -> Text {
    let source = shape(source, layout.source.width() - 16.0, fonts);
    let result = shape(&preview.text(chinese), layout.result.width() - 28.0, fonts);
    preview.text_height = result
        .layout_runs()
        .map(|run| run.line_height)
        .sum::<f32>()
        .ceil() as usize;
    preview.scroll = preview.scroll.min(
        preview
            .text_height
            .saturating_sub(layout.result.height() as usize - 12),
    );
    Text { source, result }
}

/// 【翻译测试】【正文展示】按 layout、text、metadata 和状态绘制原文、结果与错误，正文使用现有裁切和主题
pub(in crate::features::preferences::translation) fn draw(
    painter: &mut Painter<'_>,
    layout: &Layout,
    text: &Text,
    preview: &Preview,
    metadata: &str,
    chinese: bool,
) {
    let palette = Palette::current();
    let metadata = controls::fit_label(painter, metadata, 10.0, layout.bounds.width() - 28.0);
    painter.text(
        rect(
            layout.bounds.left() + 14.0,
            layout.bounds.top() + 38.0,
            layout.bounds.width() - 28.0,
            18.0,
        ),
        &metadata,
        TextStyle::new(10.0, palette.muted),
    );
    painter.text(
        rect(
            layout.source.left(),
            layout.source.top() - 23.0,
            layout.source.width(),
            20.0,
        ),
        if chinese {
            "预制原文"
        } else {
            "Sample text"
        },
        TextStyle::new(11.0, palette.muted),
    );
    let failed = matches!(preview.result, ResultState::Failed(_));
    let title = match (&preview.result, chinese) {
        (ResultState::Success(_), true) => "测试成功 · 译文",
        (ResultState::Success(_), false) => "Test succeeded · Translation",
        (ResultState::Failed(_), true) => "测试失败 · 错误详情",
        (ResultState::Failed(_), false) => "Test failed · Error details",
        (ResultState::Running, true) => "测试中",
        (ResultState::Running, false) => "Testing",
        (_, true) => "测试结果",
        (_, false) => "Test result",
    };
    let title = preview.elapsed.map_or_else(
        || title.to_owned(),
        |seconds| format!("{title} · {seconds:.2}s"),
    );
    let title = controls::fit_label(painter, &title, 11.0, layout.result.width());
    painter.text(
        rect(
            layout.result.left(),
            layout.result.top() - 25.0,
            layout.result.width(),
            22.0,
        ),
        &title,
        TextStyle::new(11.0, if failed { palette.error } else { palette.muted }),
    );
    for (bounds, buffer, offset, color) in [
        (layout.source, &text.source, 0.0, palette.text),
        (
            layout.result,
            &text.result,
            preview.scroll as f32,
            if failed { palette.error } else { palette.text },
        ),
    ] {
        controls::frame(painter, bounds, palette.window, palette.border, 6.0);
        let clip = rect(
            bounds.left() + 6.0,
            bounds.top() + 6.0,
            bounds.width() - 12.0,
            bounds.height() - 12.0,
        );
        painter.clipped_scene(Some(clip), |painter| {
            painter.text_buffer(
                buffer,
                (bounds.left() + 8.0, bounds.top() + 6.0 - offset),
                color.color(),
            )
        });
    }
    if let Some(bar) = layout.scrollbar(preview) {
        painter.fill_rounded(bar.track, 3.0, palette.alternate);
        painter.fill_rounded(bar.handle, 3.0, palette.muted);
    }
}
