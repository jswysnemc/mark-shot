//! 【滚轮提示】【输入来源】提示只读取滚轮已修改的实际属性，不产生独立历史或标注

use crate::{
    editor::{EditorState, edits::active_annotation_idx},
    features::wheel_preview::model::{Kind, Sample},
    types::AnnotationShape,
    ui::settings_panel::{SettingsWidget, StepperRange},
};

/// 【滚轮提示】【显示保护】按state返回是否处于绘制、控件或图外位置，返回布尔值，不改变交互
pub(crate) fn blocked(state: &EditorState) -> bool {
    state.selection.zone.is_none()
        || state.input.mouse_down
        || state.tool_active
        || state.ann_drag.is_some()
        || state.toolbar_drag.is_some()
        || state.navigation.pan.is_some()
        || !matches!(
            super::ui_hit::hit_test_ui(state, state.input.pointer.local),
            super::ui_hit::UiHit::None
        )
        || crate::tools::drawing_space::DrawingSpace::from_editor(state)
            .is_none_or(|space| space.start_point(state.canvas_point()).is_none())
}

/// 【滚轮提示】【属性快照】按state返回当前工具或选中标注的真实数值、颜色和原图比例，无法调整时返回空值
pub(super) fn capture(state: &EditorState) -> Option<Sample> {
    if blocked(state) {
        return None;
    }
    let range = state.settings_panel.widgets.iter().find_map(|widget| {
        if let SettingsWidget::Stepper { range, .. } = widget {
            Some(*range)
        } else {
            None
        }
    });
    let text = state.text_session.is_some() || state.settings_panel.text_layout.is_some();
    let range = if text { StepperRange::Font } else { range? };
    let selected = active_annotation_idx(state).and_then(|index| state.annotations.get(index));
    let number = selected.map_or(
        state.selected_tool == crate::tools::Tool::Number,
        |annotation| matches!(annotation.shape, AnnotationShape::Number { .. }),
    );
    let kind = match range {
        StepperRange::Font => Kind::Text,
        StepperRange::Strength => Kind::Mosaic,
        StepperRange::Magnification => Kind::Magnification,
        _ if number => Kind::Number,
        _ => Kind::Stroke,
    };
    let space = crate::tools::drawing_space::DrawingSpace::from_editor(state)?;
    Some(Sample {
        kind,
        value: if text {
            crate::editor::text_edits::current_points(state)
        } else {
            crate::app::panels::settings::stepper_value(state, range)
        },
        color: crate::app::panels::settings::current_color(state),
        image_scale: state.canvas_transform().scale as f32 / space.pixel_scale,
    })
}
