//! 【选区尺寸】【尺寸应用】精确修改以左上角为锚点，受限尺寸回写字段，拒绝不足八像素的选区

use crate::{
    editor::{EditorState, selection_geometry::ImageSpace},
    features::selection_size::{
        geometry,
        ratio::{self, Field},
    },
};

/// 【选区尺寸】【实际尺寸】按 state 返回归一化图像像素与上限，空选区返回空值
pub(in crate::app) fn dimensions(state: &EditorState) -> Option<((i32, i32), (i32, i32))> {
    let space = ImageSpace::for_editor(state)?;
    let image = space.image(state.selection.zone?)?;
    Some((
        (image.width.round() as i32, image.height.round() as i32),
        space.size,
    ))
}

/// 【选区尺寸】【可用选区】按 state 原始图像尺寸判断八像素下限，不以显示取整后的数值决定资格
pub(in crate::app) fn usable(state: &EditorState) -> bool {
    ImageSpace::for_editor(state)
        .and_then(|space| state.selection.zone.and_then(|zone| space.image(zone)))
        .is_some_and(|image| image.width >= 8.0 && image.height >= 8.0)
}

/// 【选区尺寸】【精确修改】按 requested 调整 state，dirty 返回旧新区域；返回是否接受，最小尺寸限制不写入历史
pub(in crate::app) fn size(
    state: &mut EditorState,
    requested: (i32, i32),
    dirty: &mut u32,
) -> bool {
    let Some(space) = ImageSpace::for_editor(state) else {
        return false;
    };
    let Some(current) = state.selection.zone.and_then(|zone| space.image(zone)) else {
        return false;
    };
    if current.width < 8.0 || current.height < 8.0 {
        return false;
    }
    let resized = geometry::resize(current, requested, space.size);
    if resized.width < 8.0 || resized.height < 8.0 {
        state.toasts.show_text(
            crate::ui::toast::ToastKind::SelectionTooSmall,
            crate::ui::language::Language::current()
                .text("Selection is too small")
                .into(),
            &mut state.text.font_system,
        );
        super::panels::update(state, dirty);
        crate::editor::dirty::mark_dirty(dirty, state.input.pointer.monitor_idx);
        return false;
    }
    if let Some(next) = space.global(resized) {
        state.selection.zone = Some(next);
        crate::editor::dirty::mark_all_dirty(dirty, state.placements.len());
        super::super::input::refresh_panels(state, dirty);
        return true;
    }
    false
}

/// 【选区尺寸】【字段应用】按 field 和 value 请求当前比例联动尺寸，dirty 返回实际变化，无返回值
pub(in crate::app) fn field(state: &mut EditorState, field: Field, value: i32, dirty: &mut u32) {
    let requested = state
        .selection_size
        .panel
        .as_ref()
        .map(|panel| panel.requested(field, value));
    if let Some(requested) = requested {
        size(state, requested, dirty);
    }
    super::panels::update(state, dirty);
}

/// 【选区尺寸】【比例应用】按 value 在 state 保存会话比例，并以当前宽度联动高度，dirty 返回界面与选区变化，无返回值
pub(in crate::app) fn ratio(state: &mut EditorState, value: f64, dirty: &mut u32) {
    state.selection.aspect_ratio = if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    };
    if state.selection.aspect_ratio > 0.0
        && let Some((values, maximum)) = dimensions(state)
    {
        size(
            state,
            ratio::linked(values, Field::Width, state.selection.aspect_ratio, maximum),
            dirty,
        );
    }
    super::panels::update(state, dirty);
}

/// 【选区尺寸】【字段提交】提交 state 当前活动字段，dirty 返回立即应用的尺寸；空值只结束输入，无参数以外返回值
pub(in crate::app) fn commit(state: &mut EditorState, dirty: &mut u32) {
    let request = state.selection_size.panel.as_mut().and_then(|panel| {
        let field = panel.editing()?;
        panel.field_mut(field).commit().map(|value| (field, value))
    });
    if let Some((side, value)) = request {
        field(state, side, value, dirty);
    }
    super::panels::damage(state, dirty);
    super::caret::tick(state, std::time::Instant::now(), dirty);
}
