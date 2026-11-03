//! 【文字属性栏】【鼠标事务】手柄拖动和轨道翻页每次实际数值变化分别记录历史，按钮按同位置释放处理

use crate::{
    editor::{EditorState, text_edits},
    ui::text_properties::layout::{Control, TextLayout},
};
use std::time::{Duration, Instant};

use crate::ui::text_properties::state::Press;

/// 【文字属性栏】【控件定位】返回 state 实际指针命中的文字控件，其他显示器或非文字属性栏返回空值
pub fn hit(state: &EditorState) -> Option<Control> {
    if state.input.pointer.monitor_idx != state.settings_panel.monitor_idx
        || state.settings_panel.text_layout.is_none()
    {
        return None;
    }
    state
        .settings_panel
        .hit_test(state.input.pointer.local)
        .1
        .and_then(Control::from_index)
}

/// 【文字属性栏】【数值范围】返回 control 对应的当前整数值和有效范围，非滑条控件使用零范围
pub(super) fn value(state: &EditorState, control: Control) -> (i32, (i32, i32)) {
    match control {
        Control::Size => (
            text_edits::current_points(state).round() as i32 - 19,
            (-11, 281),
        ),
        Control::Opacity => (
            (super::super::settings::current_color(state).alpha() * 100.0).round() as i32,
            (0, 100),
        ),
        _ => (0, (0, 0)),
    }
}

/// 【文字属性栏】【数值应用】把 control 的 new_value 写入 state；dirty 返回新旧标注与控件范围，每次实际变化记录历史
pub(super) fn apply(state: &mut EditorState, control: Control, new_value: i32, dirty: &mut u32) {
    if control == Control::Size {
        text_edits::set_control_points(state, (new_value + 19) as f32, dirty);
    } else if control == Control::Opacity {
        let mut color = super::super::settings::current_color(state);
        let alpha = (new_value.clamp(0, 100) as f32 * 255.0 / 100.0).round() / 255.0;
        color.set_alpha(alpha);
        let old = state.color_popover.target;
        state.color_popover.target = crate::ui::color_popover::ColorTarget::Main;
        crate::editor::color_selection::apply_color_opacity(state, color, true, dirty);
        state.color_popover.target = old;
    }
    super::super::settings::update_settings_panel(state, dirty);
}

/// 【文字属性栏】【左键按下】按 state 当前指针开始滑条或按钮手势；dirty 返回变化，返回是否消费
pub fn press(state: &mut EditorState, dirty: &mut u32) -> bool {
    let Some(control) = hit(state) else {
        return false;
    };
    if matches!(control, Control::Color | Control::Background) {
        return false;
    }
    let cell = state.settings_panel.widget_rect(control.index()).unwrap();
    let (current, range) = value(state, control);
    let x = state.input.pointer.local.0 as f32;
    let handle = TextLayout::slider_handle(cell, current, range);
    let slider = matches!(control, Control::Size | Control::Opacity);
    let drag_offset =
        (slider && x >= handle.left() && x < handle.right()).then_some(x - handle.left());
    let page = if slider && drag_offset.is_none() {
        if x < handle.left() { -1 } else { 1 }
    } else {
        0
    };
    state.input.text_property_press = Some(Press {
        control,
        monitor: state.settings_panel.monitor_idx,
        source: state.settings_panel.active_source,
        drag_offset,
        page,
        next: Instant::now() + Duration::from_millis(500),
    });
    if page != 0 {
        apply(
            state,
            control,
            (current + page * 10).clamp(range.0, range.1),
            dirty,
        );
    }
    true
}

/// 【文字属性栏】【手柄移动】把 state 指针位置用于当前抓取手柄；dirty 返回变化，返回是否存在滑条手势
pub fn pointer_move(state: &mut EditorState, dirty: &mut u32) -> bool {
    let Some(press) = &state.input.text_property_press else {
        return false;
    };
    if !matches!(press.control, Control::Size | Control::Opacity) {
        return false;
    }
    if let Some(offset) = press.drag_offset {
        let control = press.control;
        let monitor = press.monitor;
        let cell = state.settings_panel.widget_rect(control.index()).unwrap();
        let x = state.input.pointer.global.0 as f32 - state.placements[monitor].position.0 as f32;
        let (_, range) = value(state, control);
        apply(
            state,
            control,
            TextLayout::slider_value(cell, x - offset, range),
            dirty,
        );
    }
    true
}

/// 【文字属性栏】【轨道重复】按 now 重复 state 的轨道翻页，dirty 返回范围；手柄到达指针后停止重复
pub fn tick(state: &mut EditorState, now: Instant, dirty: &mut u32) {
    let Some(press) = state.input.text_property_press.as_ref() else {
        return;
    };
    if press.page == 0 || now < press.next {
        return;
    }
    let control = press.control;
    let page = press.page;
    let cell = state.settings_panel.widget_rect(control.index()).unwrap();
    let (current, range) = value(state, control);
    let handle = TextLayout::slider_handle(cell, current, range);
    let x = state.input.pointer.local.0 as f32;
    if state.input.pointer.monitor_idx != press.monitor
        || (page > 0 && x <= handle.right())
        || (page < 0 && x >= handle.left())
    {
        state.input.text_property_press.as_mut().unwrap().page = 0;
        return;
    }
    state.input.text_property_press.as_mut().unwrap().next = now + Duration::from_millis(50);
    apply(
        state,
        control,
        (current + page * 10).clamp(range.0, range.1),
        dirty,
    );
}

/// 【文字属性栏】【手势结束】结束 state 中滑条事务，dirty 返回范围；取消按钮手势不执行动作，已经修改的数值仍可撤销
pub fn finish(state: &mut EditorState, activate: bool, dirty: &mut u32) -> bool {
    let Some(press) = state.input.text_property_press.take() else {
        return false;
    };
    if !matches!(press.control, Control::Size | Control::Opacity)
        && activate
        && press.monitor == state.input.pointer.monitor_idx
        && press.source == state.settings_panel.active_source
        && hit(state) == Some(press.control)
    {
        match press.control {
            Control::Font => super::font_popup::toggle(state, dirty),
            Control::Edit => {
                if let Some(index) = state.selected_annotation {
                    let id = state.annotations[index].id;
                    crate::editor::text_session::begin_existing(state, id, dirty);
                }
            }
            _ => {}
        }
    }
    true
}

/// 【文字属性栏】【字号快捷调整】将 steps 点增量用于 state 当前文字，dirty 返回变化，无返回值，不依赖旧步进控件索引
pub fn adjust_points(state: &mut EditorState, steps: f32, dirty: &mut u32) {
    let points = text_edits::current_points(state) + steps;
    text_edits::set_control_points(state, points, dirty);
    super::super::settings::update_settings_panel(state, dirty);
}
