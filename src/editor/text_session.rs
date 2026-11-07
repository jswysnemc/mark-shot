//! 【文字事务】【生命周期】独立预览不进入持久标注，提交时保留身份、裁掉尾部空白并记录一次标注历史

use super::text_session_history::History;
use crate::{
    editor::{
        EditorState,
        annotation_drag::rebuild_annotation,
        dirty::{damage_annotation, mark_all_dirty},
    },
    tools::Tool,
    types::{Annotation, AnnotationShape, TextEditState},
};
use cosmic_text::{Edit, Selection};

pub const PREVIEW_ID: u64 = u64::MAX;

/// 【文字事务】【纯文本规范】按 text 返回 Qt 纯文本表示，将不换行空格转为空格，将段落与行分隔符转为换行
pub fn plain_text(text: &str) -> String {
    text.replace('\u{a0}', " ")
        .replace(['\u{2028}', '\u{2029}'], "\n")
}

pub struct Session {
    pub id: u64,
    pub composition: Option<super::text_composition::Composition>,
    pub caret: crate::features::text::caret::Caret,
    pub working: Annotation,
    pub source: super::text_session_format::Source,
    pub original_id: Option<u64>,
    pub history: History,
    pub dragging: bool,
    pub properties_hidden: bool,
}

impl Session {
    /// 【文字事务】【显示预览】返回候选显示或已确认草稿，不把候选文本同步到持久正文，无参数
    pub fn presented(&self) -> &Annotation {
        self.composition
            .as_ref()
            .map_or(&self.working, |composition| &composition.annotation)
    }
}

/// 【文字事务】【编辑开始】用 annotation 开始 state 的独立预览，original_id 指向已有对象；dirty 返回重绘范围，无返回值
fn begin(
    state: &mut EditorState,
    mut annotation: Annotation,
    original_id: Option<u64>,
    dirty: &mut u32,
) {
    commit(state, dirty);
    annotation.id = PREVIEW_ID;
    state.text.editors.remove(&PREVIEW_ID);
    let editor = crate::tools::text::ensure_text_editor(
        &annotation,
        &mut state.text.editors,
        &mut state.text.font_system,
    );
    editor.set_auto_indent(false);
    crate::tools::text::update_text_bbox_inline(
        &mut annotation,
        editor,
        &mut state.text.font_system,
    );
    if matches!(&annotation.shape, AnnotationShape::Text { content, .. } if content.is_empty()) {
        annotation.bbox = tiny_skia::Rect::from_xywh(
            annotation.bbox.left(),
            annotation.bbox.top(),
            annotation.bbox.width() + 1.0,
            annotation.bbox.height(),
        )
        .unwrap();
    }
    let end = editor.with_buffer(|buffer| {
        let last = buffer.lines.len().saturating_sub(1);
        cosmic_text::Cursor::new(last, buffer.lines[last].text().len())
    });
    let history = History::new(
        end,
        super::text_session_format::Font::read(&annotation),
        original_id.is_some(),
    );
    state.text_session = Some(Session {
        id: crate::features::text::input_method::focus::identity(),
        composition: None,
        caret: Default::default(),
        source: original_id
            .and_then(|id| state.annotations.iter().find(|value| value.id == id))
            .map_or_else(
                || super::text_session_format::Source::read(&annotation),
                super::text_session_format::Source::read,
            ),
        working: annotation,
        original_id,
        history,
        dragging: false,
        properties_hidden: original_id.is_some(),
    });
    state.text.editing = Some(TextEditState {
        annotation_id: PREVIEW_ID,
    });
    state.annotations_dirty = true;
    mark_all_dirty(dirty, state.placements.len());
}

/// 【文字事务】【新建开始】按 state 的默认属性、position 与 size_scale 创建编辑预览，dirty 返回范围；不会分配持久编号或记录历史
pub fn begin_new(state: &mut EditorState, position: (f32, f32), size_scale: f32, dirty: &mut u32) {
    let tools = &state.tool_settings;
    let annotation = Annotation {
        style_scale: size_scale,
        rotation_degrees: 0.0,
        frozen_bounds: crate::tools::drawing_space::DrawingSpace::from_editor(state)
            .map(|space| space.bounds),
        id: PREVIEW_ID,
        shape: AnnotationShape::Text {
            frame: Default::default(),
            start: position,
            content: String::new(),
            font_size: tools.font_size,
            bold: tools.bold,
            italic: tools.italic,
            outline: tools.outline.then_some(tools.outline_color),
            appearance: crate::features::text::TextAppearance {
                family: tools.text_family.clone(),
                background: tools.text_background,
                size_scale,
            },
        },
        color: tools.color,
        shadow_color: crate::renderer::shadow_color_for(tools.color),
        stroke_width: 0.0,
        bbox: tiny_skia::Rect::from_xywh(position.0, position.1, 1.0, 1.0).unwrap(),
    };
    state.select_annotation_index(None);
    begin(state, annotation, None, dirty);
}

/// 【文字事务】【重新编辑】在 state 中编辑 id 对应文字，dirty 返回范围；不改变当前工具，返回对象是否有效
pub fn begin_existing(state: &mut EditorState, id: u64, dirty: &mut u32) -> bool {
    commit(state, dirty);
    let Some(index) = state.annotations.iter().position(|annotation| {
        annotation.id == id && matches!(annotation.shape, AnnotationShape::Text { .. })
    }) else {
        return false;
    };
    let mut annotation = state.annotations[index].clone();
    super::text_session_layout::begin_existing(&mut annotation);
    state.select_annotation_index(Some(index));
    begin(state, annotation, Some(id), dirty);
    true
}

/// 【文字事务】【样式同步】只把 state 实际属性来源变化交给预览，dirty 返回新旧范围；面板刷新不覆盖局部撤销后的字体
pub fn sync_style(state: &mut EditorState, dirty: &mut u32) {
    super::text_session_format::sync(state, false, dirty);
}

/// 【文字事务】【正文与边界】从 state 的独立编辑器同步预览正文并重排，dirty 返回重绘区域，不写入持久标注
pub fn refresh(state: &mut EditorState, dirty: &mut u32) {
    let Some(session) = state.text_session.as_mut() else {
        return;
    };
    session.caret.activity();
    damage_annotation(
        &mut state.damage_rects,
        &mut state.layer_damage_rects,
        &session.working,
    );
    let Some(editor) = state.text.editors.get_mut(&PREVIEW_ID) else {
        return;
    };
    let text = editor.with_buffer(|buffer| {
        buffer
            .lines
            .iter()
            .map(|line| line.text())
            .collect::<Vec<_>>()
            .join("\n")
    });
    if let AnnotationShape::Text { content, .. } = &mut session.working.shape {
        *content = text;
    }
    crate::tools::text::update_text_bbox_inline(
        &mut session.working,
        editor,
        &mut state.text.font_system,
    );
    if matches!(&session.working.shape, AnnotationShape::Text { content, .. } if content.is_empty())
    {
        let rect = session.working.bbox;
        session.working.bbox =
            tiny_skia::Rect::from_xywh(rect.left(), rect.top(), rect.width() + 1.0, rect.height())
                .unwrap();
    }
    damage_annotation(
        &mut state.damage_rects,
        &mut state.layer_damage_rects,
        &session.working,
    );
    mark_all_dirty(dirty, state.placements.len());
}

/// 【文字事务】【最终提交】提交 state 的独立文字预览，dirty 返回重绘范围；空新文字不入历史，空已有文字删除，返回是否存在编辑事务
pub fn commit(state: &mut EditorState, dirty: &mut u32) -> bool {
    super::text_composition::cancel(state, dirty);
    crate::app::text_context_menu::close(state, dirty);
    if state.text_session.is_none() {
        return false;
    }
    sync_style(state, dirty);
    let wrapped_width =
        state.text_session.as_ref().and_then(|session| {
            state.text.editors.get(&PREVIEW_ID).and_then(|editor| {
                super::text_session_layout::wrapped_width(&session.working, editor)
            })
        });
    super::text_session_format::prepare_commit(state);
    let mut session = state.text_session.take().unwrap();
    damage_annotation(
        &mut state.damage_rects,
        &mut state.layer_damage_rects,
        &session.working,
    );
    state.text.editing = None;
    state.text.editors.remove(&PREVIEW_ID);
    let AnnotationShape::Text { content, .. } = &mut session.working.shape else {
        unreachable!()
    };
    *content = plain_text(content)
        .trim_end_matches(char::is_whitespace)
        .to_owned();
    let empty = content.is_empty();
    if !empty {
        super::text_session_layout::finish(
            &mut session.working,
            wrapped_width,
            &mut state.text.font_system,
        );
    }
    // 1. 【文字事务】【已有提交】持久样式已经独立记录，正文提交只替换当前对象并保留原身份
    if let Some(id) = session.original_id {
        if let Some(index) = state
            .annotations
            .iter()
            .position(|annotation| annotation.id == id)
        {
            if empty {
                state.push_undo();
                let removed = state.annotations.remove(index);
                damage_annotation(
                    &mut state.damage_rects,
                    &mut state.layer_damage_rects,
                    &removed,
                );
                state.text.editors.remove(&id);
                state.select_annotation_index(None);
            } else {
                session.working.id = id;
                let editor = crate::tools::text::ensure_text_editor(
                    &session.working,
                    &mut state.text.editors,
                    &mut state.text.font_system,
                );
                editor.set_selection(Selection::None);
                crate::tools::text::update_text_bbox_inline(
                    &mut session.working,
                    editor,
                    &mut state.text.font_system,
                );
                if !state.annotations[index].same_edit_state(&session.working) {
                    state.push_undo();
                    state.annotations[index] = session.working;
                    rebuild_annotation(state, index);
                }
                state.select_annotation_index((state.selected_tool == Tool::Pick).then_some(index));
            }
        }
    // 2. 【文字事务】【新建提交】只有非空成稿才分配编号和保存全局撤销快照
    } else if !empty {
        state.push_undo();
        session.working.id = state.next_id;
        state.next_id += 1;
        state.annotations.push(session.working);
        rebuild_annotation(state, state.annotations.len() - 1);
        state.select_annotation_index(None);
    }
    state.annotations_dirty = true;
    mark_all_dirty(dirty, state.placements.len());
    true
}
