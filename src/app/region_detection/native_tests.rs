//! 【截图控件测试】【真实输入分派】用冻结系统路径验证Tab、滚轮、异步补充和点击锁定

use super::*;
use crate::{
    app::region_detection::{native, native::Native},
    backend::window_detection::requests::Hit,
    keys::{self, Mods},
};

/// 【截图控件测试】【窗口边界】无参数，返回与编辑器冻结窗口一致的物理范围
fn window() -> PixelRect {
    PixelRect::new(40, 40, 360, 250)
}
/// 【截图控件测试】【系统路径】无参数，返回窗口内实际组件的模拟系统边界
fn path() -> Vec<PixelRect> {
    vec![
        PixelRect::new(120, 100, 40, 30),
        PixelRect::new(100, 80, 140, 100),
        window(),
    ]
}
/// 【截图控件测试】【原生会话】无参数，创建可接入系统结果的编辑器，返回不访问外部系统的状态
fn native_editor() -> EditorState {
    let mut state = editor(true, true, 1.0);
    state.region_detection.native = Some(Native::fixture(vec![(1, window())]));
    pointer(&mut state, 130.0, 120.0, &mut 0);
    state
}
/// 【截图控件测试】【结果交付】按state、path与refined通过正式结果接入交付，返回值保存在编辑器
fn result(state: &mut EditorState, path: Vec<PixelRect>, refined: bool) {
    native::apply(
        state,
        Hit {
            point: (130, 120),
            path,
            refined,
        },
        &mut 0,
    );
}
/// 【截图控件测试】【Tab按键】按state通过正式键盘分派发送无修饰Tab，返回值保存在编辑器
fn tab(state: &mut EditorState) {
    crate::app::input::handle_key(
        state,
        Some(keys::Chord {
            key: keys::Key::Tab,
            mods: Mods::default(),
        }),
        None,
        &mut 0,
    );
}

/// 【截图控件测试】【系统优先】无参数，验证系统边界优于图像候选，滚轮保持真实祖先顺序
#[test]
fn native_controls_override_image_candidates_and_wheel_uses_ancestors() {
    let mut state = native_editor();
    result(&mut state, path(), false);
    assert_eq!(
        state.region_detection.hovered,
        Rect::from_xywh(120.0, 100.0, 40.0, 30.0)
    );
    crate::app::input::handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut 0);
    assert_eq!(
        state.region_detection.hovered,
        Rect::from_xywh(100.0, 80.0, 140.0, 100.0)
    );
    crate::app::input::handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut 0);
    assert_eq!(
        state.region_detection.hovered,
        Rect::from_xywh(40.0, 40.0, 360.0, 250.0)
    );
    crate::app::input::handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut 0);
    assert_eq!(
        state.region_detection.hovered,
        Rect::from_xywh(40.0, 40.0, 360.0, 250.0)
    );
}

/// 【截图控件测试】【图像回退】无参数，验证系统组件尚未返回时缩小操作保持图像候选，扩大选窗后新组件不覆盖用户选择
#[test]
fn wheel_before_provider_reply_keeps_image_fallback_and_explicit_window() {
    let mut state = native_editor();
    let image = state.region_detection.hovered;
    crate::app::input::handle_axis_scroll(&mut state, ScrollEvent::wheel(-120), &mut 0);
    assert_eq!(state.region_detection.hovered, image);
    crate::app::input::handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut 0);
    let selected = Rect::from_xywh(40.0, 40.0, 360.0, 250.0);
    assert_eq!(state.region_detection.hovered, selected);
    result(&mut state, path(), false);
    assert_eq!(state.region_detection.hovered, selected);
}

/// 【截图控件测试】【快速整窗】无参数，验证组件未返回前Tab立即选窗，随后返回的组件不改变整窗模式
#[test]
fn tab_selects_window_before_provider_reply_and_restores_components() {
    let mut state = native_editor();
    tab(&mut state);
    assert_eq!(
        state.region_detection.hovered,
        Rect::from_xywh(40.0, 40.0, 360.0, 250.0)
    );
    result(&mut state, path(), false);
    assert_eq!(
        state.region_detection.hovered,
        Rect::from_xywh(40.0, 40.0, 360.0, 250.0)
    );
    tab(&mut state);
    assert_eq!(
        state.region_detection.hovered,
        Rect::from_xywh(120.0, 100.0, 40.0, 30.0)
    );
}

/// 【截图控件测试】【补充身份】无参数，验证新叶子插入后保留滚轮选择的原矩形，重复结果不跳回默认
#[test]
fn refinement_does_not_replace_user_selected_container() {
    let mut state = native_editor();
    result(&mut state, path(), false);
    crate::app::input::handle_axis_scroll(&mut state, ScrollEvent::wheel(120), &mut 0);
    let selected = state.region_detection.hovered;
    let mut refined = path();
    refined.insert(0, PixelRect::new(128, 118, 10, 10));
    result(&mut state, refined.clone(), true);
    assert_eq!(state.region_detection.hovered, selected);
    result(&mut state, refined, false);
    assert_eq!(state.region_detection.hovered, selected);
}

/// 【截图控件测试】【点击锁定】无参数，验证鼠标按下后补充结果和Tab均不改变采纳范围
#[test]
fn press_freezes_candidate_until_release() {
    let mut state = native_editor();
    result(&mut state, path(), false);
    let selected = state.region_detection.hovered;
    button(&mut state, MouseButton::Left, true, &mut 0);
    let mut refined = path();
    refined.insert(0, PixelRect::new(128, 118, 10, 10));
    result(&mut state, refined, true);
    tab(&mut state);
    assert_eq!(state.region_detection.hovered, selected);
    button(&mut state, MouseButton::Left, false, &mut 0);
    assert_eq!(state.selection.zone, selected);
    assert!(state.region_detection.hovered.is_none());
}

/// 【截图控件测试】【过期与修改键】无参数，验证旧指针结果忽略，带修饰Tab不切换目标
#[test]
fn stale_results_and_modified_tab_do_not_change_selection() {
    let mut state = native_editor();
    result(&mut state, path(), false);
    let selected = state.region_detection.hovered;
    crate::app::input::handle_key(
        &mut state,
        Some(keys::Chord {
            key: keys::Key::Tab,
            mods: Mods {
                shift: true,
                ..Mods::default()
            },
        }),
        None,
        &mut 0,
    );
    assert_eq!(state.region_detection.hovered, selected);
    pointer(&mut state, 250.0, 200.0, &mut 0);
    let moved = state.region_detection.hovered;
    result(&mut state, path(), false);
    assert_eq!(state.region_detection.hovered, moved);
}
