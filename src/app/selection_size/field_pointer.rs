//! 【选区尺寸】【字段鼠标】独立识别点击序列，用真实字段位置处理字符选择、词语选择与跨输出拖选

use super::field_clicks::Kind;
use crate::{
    editor::EditorState,
    features::selection_size::ratio::Field,
    ui::selection_size::{field_draw, navigation},
};

#[derive(Clone, Copy)]
pub(crate) struct Drag {
    field: Field,
    identity: u64,
}

/// 【选区尺寸】【字段坐标】按 state 与 monitor 返回指针在字段输出上的横坐标，跨输出保持同一画布坐标系
fn x(state: &EditorState, monitor: usize) -> f32 {
    (state.input.pointer.global.0 - f64::from(state.placements[monitor].position.0)) as f32
}

/// 【选区尺寸】【字符定位】按 field、extend 与 state 当前指针更新光标，dirty 返回局部变化，无返回值
fn position(state: &mut EditorState, side: Field, extend: bool, dirty: &mut u32) {
    let monitor = state.selection_size.panel.as_ref().unwrap().monitor;
    let point = x(state, monitor);
    let panel = state.selection_size.panel.as_mut().unwrap();
    let Some(rect) = panel.field_rect(side) else {
        return;
    };
    let field = &mut panel.fields[usize::from(side == Field::Height)];
    field.editor.cancel_preedit();
    let index = field_draw::index(field, rect, &mut panel.fonts, &mut panel.swash, point);
    field.edit(|editor| editor.move_to(index, extend));
    super::panels::damage(state, dirty);
    super::caret::activity(state, dirty);
}

/// 【选区尺寸】【拖选处理】按 state 中字段身份继续当前鼠标选择，dirty 返回局部变化，旧身份结束拖选而不进入画布
pub(super) fn drag(state: &mut EditorState, dirty: &mut u32) -> bool {
    let Some(drag) = state.selection_size.field_drag else {
        return false;
    };
    let valid = state.selection_size.panel.as_ref().is_some_and(|panel| {
        panel.rect.is_some()
            && panel.field(drag.field).editing
            && panel.field(drag.field).session == drag.identity
    });
    if valid {
        position(state, drag.field, true, dirty);
    } else {
        state.selection_size.field_drag = None;
    }
    true
}

/// 【选区尺寸】【字段按下】按 side 和 button 在 state 聚焦、定位或选择词语，dirty 返回实际变化；首次显示输入时保持全文选择
pub(super) fn press(
    state: &mut EditorState,
    side: Field,
    button: crate::types::MouseButton,
    dirty: &mut u32,
) {
    let was_editing = state
        .selection_size
        .panel
        .as_ref()
        .unwrap()
        .field(side)
        .editing;
    if !was_editing {
        super::keyboard::focus(state, side, dirty);
        return;
    }
    let identity = state
        .selection_size
        .panel
        .as_ref()
        .unwrap()
        .field(side)
        .session;
    let rect = state
        .selection_size
        .panel
        .as_ref()
        .unwrap()
        .field_rect(side)
        .unwrap();
    let point = state.input.pointer.local;
    let count = state.selection_size.clicks.field(
        side,
        (
            point.0 - f64::from(rect.left()),
            point.1 - f64::from(rect.top()),
        ),
        std::time::Instant::now(),
        button,
    );
    // 1. 【选区尺寸】【选择模式】单击定位，双击选择当前字符所在数字段，三击选择全部正文
    if matches!(count, Kind::Triple) {
        state
            .selection_size
            .panel
            .as_mut()
            .unwrap()
            .field_mut(side)
            .editor
            .select_all();
    } else if matches!(count, Kind::Double) && button == crate::types::MouseButton::Left {
        let monitor = state.selection_size.panel.as_ref().unwrap().monitor;
        let point = x(state, monitor);
        let panel = state.selection_size.panel.as_mut().unwrap();
        let rect = panel.field_rect(side).unwrap();
        let field = &mut panel.fields[usize::from(side == Field::Height)];
        field.editor.cancel_preedit();
        let character =
            field_draw::character(field, rect, &mut panel.fonts, &mut panel.swash, point);
        let (start, end) = navigation::word(&field.editor.text, character);
        field.edit(|editor| {
            editor.move_to(start, false);
            editor.move_to(end, true);
        });
    } else if !matches!(count, Kind::Double) {
        position(state, side, state.input.shift, dirty);
    }
    // 2. 【选区尺寸】【拖选身份】持有本次编辑身份，字段关闭后不能继续修改旧字段，程序改写保留拖选
    state.selection_size.field_drag = (button == crate::types::MouseButton::Left).then_some(Drag {
        field: side,
        identity,
    });
    super::panels::damage(state, dirty);
    super::caret::activity(state, dirty);
}
