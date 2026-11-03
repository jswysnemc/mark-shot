//! 【文字菜单】【输入拦截】菜单独占鼠标、按键和滚轮，不把外部点击传给画布

use crate::{app::text_context_menu as menu, editor::EditorState, keys::Chord, types::MouseButton};

/// 【文字菜单】【指针拦截】按 state 指针更新菜单，dirty 返回范围；返回是否存在菜单，禁止文字拖选与工具悬停穿透
pub fn pointer(state: &mut EditorState, dirty: &mut u32) -> bool {
    let Some(menu) = state.text_context_menu.as_mut() else {
        return false;
    };
    let event = menu.pointer(state.input.pointer.monitor_idx, state.input.pointer.local);
    menu::event(state, event, dirty);
    true
}

/// 【文字菜单】【按钮拦截】处理 state 上 button、pressed，dirty 返回范围；文字区右键打开菜单，外部点击只关闭菜单
pub fn button(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty: &mut u32,
) -> bool {
    if let Some(menu) = state.text_context_menu.as_mut() {
        let event = menu.button(
            state.input.pointer.monitor_idx,
            state.input.pointer.local,
            button,
            pressed,
        );
        menu::event(state, event, dirty);
        return true;
    }
    if pressed
        && matches!(button, MouseButton::Right)
        && crate::tools::text::session_input::inside(state)
        && matches!(
            super::ui_hit::hit_test_ui(state, state.input.pointer.local),
            super::ui_hit::UiHit::None
        )
    {
        menu::open(state, dirty);
        return true;
    }
    false
}

/// 【文字菜单】【键盘拦截】处理 state 菜单中 chord，dirty 返回范围；菜单可见时所有按键都不进入正文或全局快捷键
pub fn key(state: &mut EditorState, chord: Option<Chord>, dirty: &mut u32) -> bool {
    let Some(menu) = state.text_context_menu.as_mut() else {
        return false;
    };
    if let Some(chord) = chord {
        let event = menu.key(chord.key, chord.mods.shift);
        menu::event(state, event, dirty);
    }
    true
}
