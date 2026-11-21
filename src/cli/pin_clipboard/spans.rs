//! 【贴图】【文本片段】共用 HTML 和 RTF 文本片段的合并与空白裁剪

use super::html::{Span, SpanStyle};

/// 【贴图】【片段追加】按 spans 与 text 追加片段，相同样式与上一片段合并
pub(super) fn append_span(spans: &mut Vec<Span>, text: &str, style: &SpanStyle) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = spans.last_mut()
        && last.style == *style
    {
        last.text.push_str(text);
        return;
    }
    spans.push(Span {
        text: text.into(),
        style: style.clone(),
    });
}

/// 【贴图】【结尾裁剪】按 code 裁剪片段尾部空白，代码块只移除一个尾随换行
pub(super) fn finalize_spans(spans: &mut Vec<Span>, code: bool) {
    if code {
        if let Some(last) = spans.last_mut() {
            if last.text.ends_with("\r\n") {
                last.text.truncate(last.text.len() - 2);
            } else if last.text.ends_with('\n') {
                last.text.pop();
            }
        }
    } else {
        for span in spans.iter_mut().rev() {
            let trimmed = span
                .text
                .trim_end_matches(|ch: char| ch.is_ascii_whitespace());
            span.text.truncate(trimmed.len());
            if !span.text.is_empty() {
                break;
            }
        }
    }
    spans.retain(|span| !span.text.is_empty());
}

/// 【贴图】【尾部空白】接收 spans 并裁剪普通正文尾部空白，删除空片段，无返回值
pub(super) fn trim_spans(spans: &mut Vec<Span>) {
    finalize_spans(spans, false);
}
