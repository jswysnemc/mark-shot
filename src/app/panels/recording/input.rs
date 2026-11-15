//! 【录屏工具】【菜单输入】菜单拥有完整点击序列，鼠标、键盘与滚轮不传递到选区

use crate::{
    backend::window::{KeyInput, LEFT_BUTTON, WindowEvent},
    editor::EditorState,
    keys::Chord,
    types::{MouseButton, ScrollEvent},
    ui::{
        controls::ChoiceEvent,
        settings_panel::{SettingsAction, SettingsWidget},
    },
};

/// 【录屏工具】【菜单事件】把 event 应用于 state 菜单，dirty 返回损坏；返回是否消费事件
fn event(state: &mut EditorState, event: WindowEvent, dirty: &mut u32) -> bool {
    let Some(menu) = &mut state.recording_tool.menu else {
        return false;
    };
    let size = state.placements[menu.monitor].size;
    let parameter = menu.parameter;
    let result = menu.choice.event(&event, (size.0 as u32, size.1 as u32));
    match result {
        ChoiceEvent::Selected(index) => {
            parameter.choose(index, &mut state.recording_tool.config);
            super::close(state, dirty);
            crate::app::input::refresh_panels(state, dirty);
        }
        ChoiceEvent::Closed => super::close(state, dirty),
        ChoiceEvent::Redraw => super::damage(state, dirty),
        ChoiceEvent::Keep => {}
    }
    crate::editor::dirty::apply_damage_rects(state, dirty);
    true
}

/// 【录屏工具】【菜单指针】按 state 当前屏幕指针更新候选，dirty 返回损坏；返回是否消费移动
pub(crate) fn pointer(state: &mut EditorState, dirty: &mut u32) -> bool {
    if state.recording_tool.menu.is_none() {
        return false;
    }
    let position = position(state);
    event(state, WindowEvent::PointerMove { position }, dirty)
}

/// 【录屏工具】【菜单位置】按 state 返回菜单所属屏幕内的指针，其他显示器返回菜单外坐标
fn position(state: &EditorState) -> (f64, f64) {
    if state
        .recording_tool
        .menu
        .as_ref()
        .is_some_and(|menu| menu.monitor == state.input.pointer.monitor_idx)
    {
        state.input.pointer.local
    } else {
        (-1.0, -1.0)
    }
}

/// 【录屏工具】【菜单点击】按 button、pressed 处理 state 菜单或外部释放，dirty 返回损坏；返回是否消费
pub(crate) fn button(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty: &mut u32,
) -> bool {
    if !pressed && state.recording_tool.release_owner == Some(button) {
        state.recording_tool.release_owner = None;
        return true;
    }
    if state.recording_tool.menu.is_none() {
        return false;
    }
    if button != MouseButton::Left {
        if pressed {
            super::close(state, dirty);
            state.recording_tool.release_owner = Some(button);
        }
        return true;
    }
    let position = position(state);
    event(
        state,
        WindowEvent::PointerButton {
            position,
            button: LEFT_BUTTON,
            pressed,
        },
        dirty,
    );
    if pressed && state.recording_tool.menu.is_none() {
        state.recording_tool.release_owner = Some(button);
    }
    true
}

/// 【录屏工具】【菜单键盘】按 chord 处理 state 菜单，dirty 返回损坏；打开时消费快捷键，避免导出和退出
pub(crate) fn key(state: &mut EditorState, chord: Option<Chord>, dirty: &mut u32) -> bool {
    if state.recording_tool.menu.is_none() {
        return false;
    }
    if let Some(chord) = chord {
        event(
            state,
            WindowEvent::Key(KeyInput {
                key: Some(chord.key),
                mods: chord.mods,
                text: String::new(),
            }),
            dirty,
        );
    }
    true
}

/// 【录屏工具】【参数滚轮】按 event 微调 state 帧率或倒计时；菜单打开时仅关闭，dirty 返回损坏，返回是否消费
pub(crate) fn scroll(state: &mut EditorState, event: ScrollEvent, dirty: &mut u32) -> bool {
    if state.recording_tool.menu.is_some() {
        super::close(state, dirty);
        return true;
    }
    if state.selected_tool != crate::tools::Tool::Recording {
        return false;
    }
    if state.input.pointer.monitor_idx != state.settings_panel.monitor_idx {
        return false;
    }
    let Some(SettingsWidget::Action {
        action: SettingsAction::Recording(parameter),
        ..
    }) = state
        .settings_panel
        .hit_test(state.input.pointer.local)
        .1
        .and_then(|index| state.settings_panel.widgets.get(index))
    else {
        return false;
    };
    if parameter.step(
        event.canvas_steps().round() as i32,
        &mut state.recording_tool.config,
    ) {
        crate::app::input::refresh_panels(state, dirty);
    }
    true
}
