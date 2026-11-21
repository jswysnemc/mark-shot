//! 【贴图】【HTML标签工具】提供块级判断、颜色提取与片段裁剪等无状态辅助函数

use super::super::html_style;
use super::model::{Align, Span, SpanStyle};
use super::token::Tag;
use crate::theme::Rgba;

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

/// 【贴图】【块标签】按 name 判断是否作为普通段落容器，返回是否为块级标签
pub(super) fn is_block(name: &str) -> bool {
    matches!(
        name,
        "p" | "div"
            | "section"
            | "article"
            | "header"
            | "footer"
            | "main"
            | "nav"
            | "aside"
            | "figure"
            | "figcaption"
            | "dl"
            | "dt"
            | "dd"
            | "table"
            | "thead"
            | "tbody"
            | "tfoot"
    )
}

/// 【贴图】【外层容器标签】按 name 判断是否可作为画布背景来源的容器标签，返回是否属于外层容器
pub(super) fn is_outer_container_tag(name: &str) -> bool {
    matches!(
        name,
        "html" | "body" | "div" | "main" | "section" | "article" | "pre" | "table"
    )
}

/// 【贴图】【块级背景标签】按 name 判断其背景是否应作为块背景而非内联高亮，返回是否属于块级背景
pub(super) fn is_block_background_tag(name: &str) -> bool {
    is_block(name)
        || is_heading(name)
        || matches!(name, "html" | "body" | "pre" | "blockquote" | "li" | "tr")
}

/// 【贴图】【标签背景】按 style 属性读取 background-color 或可解析为颜色的 background，返回背景色
pub(super) fn tag_background(tag: &Tag) -> Option<Rgba> {
    tag_color_declaration(tag, |property| {
        property.eq_ignore_ascii_case("background-color")
            || property.eq_ignore_ascii_case("background")
    })
}

/// 【贴图】【标签前景】按 style 属性读取 color，返回文字颜色
pub(super) fn tag_color(tag: &Tag) -> Option<Rgba> {
    tag_color_declaration(tag, |property| property.eq_ignore_ascii_case("color"))
}

/// 【贴图】【声明取色】按 style 属性与 property 匹配器返回首个可解析颜色
pub(super) fn tag_color_declaration(tag: &Tag, wanted: impl Fn(&str) -> bool) -> Option<Rgba> {
    let css = tag.attr("style")?;
    html_style::declarations(css)
        .into_iter()
        .find_map(|(property, value)| {
            if wanted(property) {
                html_style::color(value)
            } else {
                None
            }
        })
}

/// 【贴图】【标题级别】按 name 解析 h1..h6，返回标题级别
pub(super) fn heading_level(name: &str) -> Option<u8> {
    match name {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}

/// 【贴图】【标题判断】按 name 判断是否为标题标签，返回是否为标题
pub(super) fn is_heading(name: &str) -> bool {
    heading_level(name).is_some()
}

/// 【贴图】【块对齐】按 style 的 text-align 与 align 属性解析块级对齐，均缺失时返回空值
pub(super) fn tag_align(tag: &Tag) -> Option<Align> {
    if let Some(css) = tag.attr("style") {
        for (property, value) in html_style::declarations(css) {
            if property.eq_ignore_ascii_case("text-align") {
                return Some(parse_align(value));
            }
        }
    }
    tag.attr("align").map(parse_align)
}

/// 【贴图】【对齐解析】按 value 解析 left/center/right/justify，未知值回退左对齐
fn parse_align(value: &str) -> Align {
    match value.trim().to_ascii_lowercase().as_str() {
        "center" => Align::Center,
        "right" => Align::Right,
        _ => Align::Left,
    }
}
