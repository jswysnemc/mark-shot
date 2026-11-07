//! 【OCR 翻译】【提示排版】布局测量与绘制共用有界换行字形，静止窗口不重新排版

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};

/// 【OCR 翻译】【有界提示】按 fonts、text、width 返回十二像素、十七像素行高的换行字形，空提示返回空值
pub(super) fn shape(fonts: &mut FontSystem, text: &str, width: f32) -> Option<Buffer> {
    if text.is_empty() {
        return None;
    }
    let mut buffer = Buffer::new(fonts, Metrics::new(12.0, 17.0));
    buffer.set_size(Some(width), None);
    buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(fonts, false);
    Some(buffer)
}

/// 【OCR 翻译】【可见高度】按 buffer 返回至多三行的提示高度，空值高度为零，不修改完整错误信息
pub(super) fn height(buffer: Option<&Buffer>) -> f32 {
    buffer.map_or(0.0, |buffer| {
        buffer.layout_runs().count().clamp(1, 3) as f32 * 17.0
    })
}
