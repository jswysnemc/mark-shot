//! 【文字字号】【统一修改】各输入入口同步默认字号，换行框按点字号比例缩放，历史只保存标注

use crate::{
    editor::{
        EditorState,
        edits::{active_annotation_idx, commit_settings_change},
    },
    features::text::geometry,
    types::AnnotationShape,
};

/// 【文字字号】【数值应用】将 points 应用到 state 的文字或默认值；record_undo 控制历史，dirty 返回重绘范围，返回是否实际改变
pub fn set(state: &mut EditorState, points: f32, record_undo: bool, dirty: &mut u32) -> bool {
    let points = geometry::points(points);
    let index = active_annotation_idx(state);
    let wrapped = if let Some(index) = index {
        let AnnotationShape::Text { font_size, .. } = &state.annotations[index].shape else {
            return false;
        };
        if *font_size == points {
            return false;
        }
        let editor = crate::tools::text::ensure_text_editor(
            &state.annotations[index],
            &mut state.text.editors,
            &mut state.text.font_system,
        );
        cosmic_text::Edit::with_buffer(editor, |buffer| buffer.wrap() != cosmic_text::Wrap::None)
    } else {
        if state.tool_settings.font_size == points {
            return false;
        }
        false
    };
    // 1. 【文字字号】【默认同步】同值输入已经返回，只有实际修改才覆盖工具默认字号
    commit_settings_change(
        state,
        true,
        record_undo,
        |tools| tools.font_size = points,
        |annotation| {
            if let AnnotationShape::Text {
                font_size,
                appearance,
                ..
            } = &mut annotation.shape
            {
                // 2. 【文字字号】【换行缩放】原有换行按字号比例保留框宽，单行改用自然宽度
                appearance.wrap_width = if wrapped {
                    Some(
                        (appearance.wrap_width.unwrap_or(annotation.bbox.width()) * points
                            / *font_size)
                            .max(1.0),
                    )
                } else {
                    None
                };
                *font_size = points;
            }
        },
        dirty,
    );
    true
}
