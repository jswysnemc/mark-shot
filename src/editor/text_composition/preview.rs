//! 【输入法】【候选预览】候选使用独立标注和编辑缓存，只更新动态层，取消时不回填原选区

use cosmic_text::{Edit as _, Selection, UnderlineStyle};

use crate::{
    editor::{DamageZone, EditorState, text_session::PREVIEW_ID},
    features::text::input_method::positions,
    types::{Annotation, AnnotationShape, text_input::Preedit},
};

/// 【输入法】【显示损坏】把 annotation 的可见范围加入 zones，只影响动态背景，不修改持久图层，无返回值
pub(super) fn damage(annotation: &Annotation, zones: &mut Vec<DamageZone>) {
    zones.push(DamageZone::Global(annotation.damage_bbox(false)));
}

/// 【输入法】【候选取消】移除 state 的独立候选和缓存，dirty 返回旧范围，返回是否有候选；真实文档保持不变
pub(super) fn cancel(state: &mut EditorState, dirty: &mut u32) -> bool {
    let Some(composition) = state
        .text_session
        .as_mut()
        .and_then(|session| session.composition.take())
    else {
        return false;
    };
    damage(&composition.annotation, &mut state.damage_rects);
    if let Some(session) = state.text_session.as_ref() {
        damage(&session.working, &mut state.damage_rects);
    }
    state.text.editors.remove(&super::PREEDIT_ID);
    crate::editor::dirty::apply_damage_rects(state, dirty);
    true
}

/// 【输入法】【候选显示】按 state 已确认文本与 preedit 创建独立显示，dirty 返回候选范围；仅候选变动时复制显示文本，不在闪烁帧重建
pub(super) fn show(state: &mut EditorState, preedit: Preedit, dirty: &mut u32) {
    let Some(session) = state.text_session.as_mut() else {
        return;
    };
    let Some(committed) = state.text.editors.get(&PREVIEW_ID) else {
        return;
    };
    let position = committed.cursor();
    let mut annotation = session.working.clone();
    annotation.id = super::PREEDIT_ID;
    let AnnotationShape::Text { content, .. } = &mut annotation.shape else {
        return;
    };
    let Some(offset) = positions::document_offset(content, position) else {
        return;
    };
    content.insert_str(offset, &preedit.text);
    let start = positions::editor_cursor(content, offset).unwrap();
    let end = positions::editor_cursor(content, offset + preedit.text.len()).unwrap();
    let cursor = preedit.cursor.unwrap_or((0, 0));
    let begin = positions::editor_cursor(content, offset + cursor.0).unwrap();
    let anchor = positions::editor_cursor(content, offset + cursor.1).unwrap();
    let editor = crate::tools::text::ensure_text_editor(
        &annotation,
        &mut state.text.editors,
        &mut state.text.font_system,
    );
    // 1. 【输入法】【候选格式】只对候选范围加下划线，原正文格式不改变，换行按实际编辑行切分
    editor.with_buffer_mut(|buffer| {
        for line in start.line..=end.line {
            let value = &mut buffer.lines[line];
            let from = if line == start.line { start.index } else { 0 };
            let to = if line == end.line {
                end.index
            } else {
                value.text().len()
            };
            let mut attrs = value.attrs_list().clone();
            let defaults = cosmic_text::AttrsOwned::new(&attrs.defaults());
            let original: Vec<_> = attrs
                .spans_iter()
                .map(|(range, attrs)| (range.clone(), attrs.clone()))
                .collect();
            attrs.add_span(
                from..to,
                &defaults.as_attrs().underline(UnderlineStyle::Single),
            );
            // 2. 【输入法】【缺字字体】原有回退字体片段仍保留，只增加下划线，不强制候选全部使用主字体
            for (range, previous) in original {
                let left = from.max(range.start);
                let right = to.min(range.end);
                attrs.add_span(
                    left..right,
                    &previous.as_attrs().underline(UnderlineStyle::Single),
                );
            }
            value.set_attrs_list(attrs);
        }
    });
    editor.set_cursor(begin);
    editor.set_selection(if begin == anchor {
        Selection::None
    } else {
        Selection::Normal(anchor)
    });
    crate::tools::text::update_text_bbox_inline(
        &mut annotation,
        editor,
        &mut state.text.font_system,
    );
    damage(&annotation, &mut state.damage_rects);
    session.composition = Some(super::Composition {
        annotation,
        preedit,
        position,
    });
    crate::editor::dirty::apply_damage_rects(state, dirty);
}

/// 【输入法】【外观比较】比较 left、right 的文字外观，不比较候选正文与扩展后的边界；返回样式是否一致
fn same_style(left: &Annotation, right: &Annotation) -> bool {
    if left.color != right.color
        || left.shadow_color != right.shadow_color
        || left.stroke_width != right.stroke_width
    {
        return false;
    }
    match (&left.shape, &right.shape) {
        (
            AnnotationShape::Text {
                start: left_start,
                font_size: left_size,
                bold: left_bold,
                italic: left_italic,
                outline: left_outline,
                appearance: left_appearance,
                ..
            },
            AnnotationShape::Text {
                start: right_start,
                font_size: right_size,
                bold: right_bold,
                italic: right_italic,
                outline: right_outline,
                appearance: right_appearance,
                ..
            },
        ) => {
            left_start == right_start
                && left_size == right_size
                && left_bold == right_bold
                && left_italic == right_italic
                && left_outline == right_outline
                && left_appearance == right_appearance
        }
        _ => false,
    }
}

/// 【输入法】【候选样式】仅 state 实际外观变化时重建候选，dirty 返回新旧区域；文字内容和光标相位不触发全文复制，无返回值
pub(super) fn refresh_style(state: &mut EditorState, dirty: &mut u32) {
    let preedit = state.text_session.as_ref().and_then(|session| {
        let composition = session.composition.as_ref()?;
        (!same_style(&session.working, &composition.annotation))
            .then(|| composition.preedit.clone())
    });
    if let Some(preedit) = preedit {
        cancel(state, dirty);
        show(state, preedit, dirty);
    }
}
