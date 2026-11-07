//! 【文字标注】【排版同步】正文、字体样式与背景框使用同一套实际字体度量

use super::*;
use crate::features::text::geometry;

/// 【文字标注】【完整排版】将 buffer 排版到末尾；font_system 提供字形，prune 控制清理旧布局，无返回值
pub fn shape_entire_buffer(
    buffer: &mut cosmic_text::Buffer,
    font_system: &mut cosmic_text::FontSystem,
    prune: bool,
) {
    if let Some(last) = buffer.lines.last() {
        let cursor = cosmic_text::Cursor::new(buffer.lines.len() - 1, last.text().len());
        buffer.shape_until_cursor(font_system, cursor, prune);
    }
}

/// 【文字标注】【内容读取】返回 editor 的全部正文，段落之间保留一个换行符
fn content(editor: &Editor<'static>) -> String {
    editor.with_buffer(|buffer| {
        buffer
            .lines
            .iter()
            .map(|line| line.text())
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// 【文字标注】【缓存比较】比较 editor 行内容与 expected，不拼接全文或分配临时字符串，返回内容是否完全一致
fn content_matches(editor: &Editor<'static>, expected: &str) -> bool {
    editor.with_buffer(|buffer| {
        let mut remaining = expected;
        for (index, line) in buffer.lines.iter().enumerate() {
            if index != 0 {
                let Some(next) = remaining.strip_prefix('\n') else {
                    return false;
                };
                remaining = next;
            }
            let Some(next) = remaining.strip_prefix(line.text()) else {
                return false;
            };
            remaining = next;
        }
        remaining.is_empty()
    })
}

/// 【文字标注】【背景边界】按 ann 的样式与 editor 实际布局返回文字框；透明背景同样保留对称内边距
pub fn text_bounds(ann: &Annotation, editor: &Editor<'static>) -> Rect {
    let AnnotationShape::Text {
        start, appearance, ..
    } = &ann.shape
    else {
        return ann.bbox;
    };
    let (width, height) = editor.with_buffer(|buffer| {
        let mut width = 0.0_f32;
        let mut height = buffer.metrics().line_height;
        // 1. 【文字标注】【自然收框】未换行时按实际字形收框，存储矩形高度不改变文字排版高度
        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
            height = height.max(run.line_top + run.line_height);
        }
        // 2. 【文字标注】【换行宽度】预览沿用文档分配宽度，原图选框另外排除软换行尾部空格
        if buffer.wrap() != cosmic_text::Wrap::None {
            width = width.max(buffer.size().0.unwrap_or(0.0));
        }
        (width, height)
    });
    let base = ann.text_stored_bounds();
    let origin = base.map_or(*start, |bounds| (bounds.left(), bounds.top()));
    geometry::bounds(origin, width, height, appearance.size_scale)
}

/// 【文字标注】【输入同步】把 id 对应 editor 正文同步到 annotations；font_system 排版并更新背景边界，无返回值
pub(super) fn sync_content_from_editor(
    id: u64,
    editor: &mut Editor<'static>,
    annotations: &mut [Annotation],
    font_system: &mut cosmic_text::FontSystem,
) {
    editor.with_buffer_mut(|buffer| shape_entire_buffer(buffer, font_system, false));
    let text = content(editor);
    if let Some(ann) = annotations.iter_mut().find(|ann| ann.id == id) {
        if let AnnotationShape::Text { content, .. } = &mut ann.shape {
            *content = text;
        }
        synchronize_editor(ann, editor, font_system, true);
        update_bounds(ann, editor, font_system);
    }
}

/// 【文字标注】【缓存就绪】按 ann 内容与外观取得 text_editors 中的编辑器；font_system 提供字体，样式变化才重新排版
pub fn ensure_text_editor<'a>(
    ann: &Annotation,
    text_editors: &'a mut std::collections::HashMap<u64, Editor<'static>>,
    font_system: &mut cosmic_text::FontSystem,
) -> &'a mut Editor<'static> {
    let editor = text_editors
        .entry(ann.id)
        .or_insert_with(|| Editor::new(Buffer::new_empty(Metrics::new(1.0, 1.0))));
    synchronize_editor(ann, editor, font_system, false);
    editor
}

/// 【文字标注】【样式同步】用 ann 内容与外观更新 editor；font_system 提供字体，force_reflow 用于实际输入后重新检查缺字与换行，无返回值
pub(super) fn synchronize_editor(
    ann: &Annotation,
    editor: &mut Editor<'static>,
    font_system: &mut cosmic_text::FontSystem,
    force_reflow: bool,
) {
    let AnnotationShape::Text {
        content,
        frame,
        font_size,
        bold,
        italic,
        appearance,
        ..
    } = &ann.shape
    else {
        panic!("text editor requires a text annotation");
    };
    let family = crate::fonts::text_faces::family(font_system, &appearance.family, *bold, *italic);
    let pixels = geometry::pixels(*font_size, appearance.size_scale);
    let line_height =
        crate::fonts::text_faces::line_height(font_system, &family, pixels, *bold, *italic);
    let metrics = Metrics::new(pixels, line_height);
    let weight = if *bold {
        cosmic_text::Weight(600)
    } else {
        cosmic_text::Weight::NORMAL
    };
    let style = if *italic {
        cosmic_text::Style::Italic
    } else {
        cosmic_text::Style::Normal
    };
    let width = frame
        .width
        .and_then(|_| ann.text_stored_bounds())
        .and_then(|bounds| {
            let width = bounds.width();
            let available = width - geometry::padding(appearance.size_scale).0 * 2.0;
            (available >= 16.0 * appearance.size_scale).then_some(available)
        });
    let changed = editor.with_buffer(|buffer| {
        let style_differs = buffer.lines.first().is_none_or(|line| {
            let attrs = line.attrs_list().defaults();
            attrs.weight != weight || attrs.style != style || attrs.family != Family::Name(&family)
        });
        buffer.metrics() != metrics || buffer.size().0 != width || style_differs
    }) || !content_matches(editor, content);
    if changed || force_reflow {
        // 1. 【文字标注】【统一样式】字号、字重、家族和换行同步更新，避免属性改变后仍绘制旧缓存
        editor.with_buffer_mut(|buffer| {
            buffer.set_metrics(metrics);
            buffer.set_wrap(cosmic_text::Wrap::None);
            buffer.set_size(None, None);
            let attrs = Attrs::new()
                .family(Family::Name(&family))
                .weight(weight)
                .style(style);
            crate::fonts::text_spans::set_text(buffer, font_system, content, &attrs);
            shape_entire_buffer(buffer, font_system, true);
            // 2. 【文字标注】【原图换行】是否换行按原图字号判断，预览字体取整不能单独决定换行开关
            let wraps = width.is_some_and(|available| {
                let source_width = if appearance.size_scale == 1.0 {
                    natural_width(buffer)
                } else {
                    super::source_layout::natural_width(ann, font_system)
                };
                source_width > available / appearance.size_scale + 0.5
            });
            buffer.set_wrap(if wraps {
                cosmic_text::Wrap::WordOrGlyph
            } else {
                cosmic_text::Wrap::None
            });
            buffer.set_size(width, None);
            shape_entire_buffer(buffer, font_system, true);
        });
    }
}

/// 【文字标注】【自然宽度】返回 buffer 所有段落中的最大自然行宽，用于原图换行判断
pub(super) fn natural_width(buffer: &Buffer) -> f32 {
    buffer
        .layout_runs()
        .map(|run| run.line_w)
        .fold(0.0, f32::max)
}

/// 【文字标注】【双重边界】按ann、editor和font_system更新原图选框及当前字形缓存，存储宽高保持独立，无返回值
fn update_bounds(
    ann: &mut Annotation,
    editor: &Editor<'static>,
    font_system: &mut cosmic_text::FontSystem,
) {
    let painted = text_bounds(ann, editor);
    let bounds = super::source_layout::source_bounds(ann, font_system);
    ann.bbox = bounds;
    if let AnnotationShape::Text { frame, .. } = &mut ann.shape {
        frame.painted = Some(painted);
    }
}

/// 【文字标注】【边界更新】按 ann 与 editor 当前正文刷新排版及背景边界；font_system 提供字形，无返回值
pub fn update_text_bbox_inline(
    ann: &mut Annotation,
    editor: &mut Editor<'static>,
    font_system: &mut cosmic_text::FontSystem,
) {
    synchronize_editor(ann, editor, font_system, false);
    editor.with_buffer_mut(|buffer| shape_entire_buffer(buffer, font_system, true));
    update_bounds(ann, editor, font_system);
}

/// 【文字标注】【确认重排】按 ann 已确认正文重新解析字体与缺字回退，editor 保留光标和选区，font_system 提供字体，无返回值
pub fn reflow_text_editor(
    ann: &mut Annotation,
    editor: &mut Editor<'static>,
    font_system: &mut cosmic_text::FontSystem,
) {
    synchronize_editor(ann, editor, font_system, true);
    update_bounds(ann, editor, font_system);
}
