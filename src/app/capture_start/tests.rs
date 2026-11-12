//! 【截图起始测试】【输入回归】使用完整指针入口验证顶部点击与选区的输入归属

use super::*;
use crate::{
    types::MouseButton,
    ui::capture_start::{Control, layout},
};

/// 【截图录制测试】【工具栏避让】全屏选区显示编辑工具栏时，录制栏不能遮住截图工具按钮
#[test]
fn recording_bar_remains_clear_of_the_editing_toolbar() {
    let mut state = editor();
    state.capture_start.visible = false;
    state.capture_start.status.phase = "recording".into();
    state.selection.zone = state.placements[0].rect();
    state.ui_hidden = false;
    state.toolbar.monitor_idx = 0;
    state.toolbar.position = (100.0, 8.0);
    state.toolbar.render_pos = state.toolbar.position;
    state.toolbar.size = (1000.0, 40.0);
    state.toolbar.opacity = 1.0;
    for top in [8.0, 56.0] {
        state.toolbar.position.1 = top;
        state.toolbar.render_pos = state.toolbar.position;
        placement::sync(&mut state, &mut 0);
        let bar = layout::toolbar(state.placements[0].size, &state.capture_start, 0);
        let tools = tiny_skia::Rect::from_xywh(100.0, top, 1000.0, 40.0).unwrap();
        assert!(!crate::utils::rects_overlap(&bar, &tools));
    }
    let stop = layout::toolbar_controls(state.placements[0].size, &state.capture_start, 0)
        .into_iter()
        .find(|(control, _)| *control == Control::Stop)
        .unwrap()
        .1;
    crate::app::input::handle_pointer_move(
        &mut state,
        0,
        f64::from(stop.left() + stop.width() / 2.0),
        f64::from(stop.top() + stop.height() / 2.0),
        &mut 0,
    );
    for pressed in [true, false] {
        crate::app::input::handle_pointer_button(&mut state, MouseButton::Left, pressed, &mut 0);
    }
    assert!(state.capture_start.stop_requested);
}

/// 【截图起始测试】【后台停止】截图选区形成后仍能停止后台录制，不能因起始提示隐藏而丢弃控制快捷键
#[test]
fn recording_stop_survives_completed_screenshot_selection() {
    let mut state = editor();
    state.capture_start.status.phase = "recording".into();
    state.capture_start.status.window_open = true;
    click(&mut state, Control::Fullscreen);
    assert!(!state.capture_start.visible);
    state.mode = Mode::Ocr;
    crate::app::actions::run(&mut state, crate::keys::Action::RecordStop, &mut 0);
    assert!(state.capture_start.stop_requested);
}

/// 【截图起始测试】【编辑器】无参数，返回启用起始界面的轻量测试编辑器
fn editor() -> EditorState {
    let mut state = crate::app::input::tests::editor();
    initialize(&mut state, &super::super::Launch::default());
    state.capture_start.checked = true;
    state
}

/// 【截图起始测试】【按钮点击】按 state 与 control 发送控件中心的完整按下和松开
fn click(state: &mut EditorState, control: Control) {
    let bounds = layout::controls(state.placements[0].size, state.capture_start.active())
        .into_iter()
        .find(|(value, _)| *value == control)
        .unwrap()
        .1;
    crate::app::input::handle_pointer_move(
        state,
        0,
        (bounds.left() + bounds.width() / 2.0) as f64,
        (bounds.top() + bounds.height() / 2.0) as f64,
        &mut 0,
    );
    for pressed in [true, false] {
        crate::app::input::handle_pointer_button(state, MouseButton::Left, pressed, &mut 0);
    }
}

/// 【截图起始测试】【模式交接】录屏和 GIF 分别传递本次格式，切回选区恢复交互编辑，不提前创建选框
#[test]
fn mode_buttons_do_not_select_the_desktop_underneath() {
    let mut state = editor();
    for (control, format) in [(Control::Video, Format::Video), (Control::Gif, Format::Gif)] {
        click(&mut state, control);
        assert_eq!(state.capture_start.recording, Some(format));
        assert_eq!(state.mode, Mode::Region);
        assert!(state.selection.zone.is_none() && state.finish.is_none());
        assert!(!state.input.mouse_down && !state.tool_active);
    }
    click(&mut state, Control::Region);
    assert!(state.capture_start.recording.is_none());
    assert_eq!(state.mode, Mode::Editor);
    assert!(!state.ui_hidden);
}

/// 【截图起始测试】【全屏交接】点击全屏选中当前输出，永久移除起始界面，不立即保存截图
#[test]
fn fullscreen_selects_current_output_and_dismisses_intro() {
    let mut state = editor();
    click(&mut state, Control::Fullscreen);
    assert_eq!(state.selection.zone, state.placements[0].rect());
    assert!(!state.capture_start.visible);
    assert!(state.fullscreen_scope && state.finish.is_none());
    state.selection.zone = None;
    sync(&mut state, &mut 0);
    assert!(!state.capture_start.visible);
}

/// 【截图起始测试】【实际拖选】在画布拖动形成选区后，起始控件消失，释放仍到达选区工具且完成录制交接
#[test]
fn gif_selection_finishes_and_keeps_requested_format() {
    let mut state = editor();
    click(&mut state, Control::Gif);
    crate::app::input::handle_pointer_move(&mut state, 0, 80.0, 120.0, &mut 0);
    crate::app::input::handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    crate::app::input::handle_pointer_move(&mut state, 0, 400.0, 300.0, &mut 0);
    sync(&mut state, &mut 0);
    assert!(state.capture_start.visible);
    crate::app::input::handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    sync(&mut state, &mut 0);
    assert!(!state.capture_start.visible);
    assert!(state.finish.is_some());
    assert_eq!(state.capture_start.recording, Some(Format::Gif));
    assert!(!state.input.mouse_down && !state.tool_active);
}

/// 【截图起始测试】【跨区释放】在顶部按下后移出松开，不得产生选区、结束截图或残留按下状态
#[test]
fn top_press_released_on_canvas_is_consumed() {
    let mut state = editor();
    crate::app::input::handle_pointer_move(&mut state, 0, 320.0, 30.0, &mut 0);
    crate::app::input::handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    crate::app::input::handle_pointer_move(&mut state, 0, 400.0, 300.0, &mut 0);
    crate::app::input::handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    assert!(state.selection.zone.is_none() && state.finish.is_none());
    assert!(state.capture_start.pressed.is_none());
    assert!(!state.input.mouse_down && !state.tool_active);
}

/// 【截图起始测试】【活动保护】已有录制时入口被控制按钮替换，快捷键也不能开始新的会话，仍允许截图
#[test]
fn active_recording_does_not_start_another_session() {
    let mut state = editor();
    state.capture_start.status.phase = "recording".into();
    state.capture_start.status.window_open = true;
    assert!(
        layout::controls(state.placements[0].size, true)
            .iter()
            .all(|(control, _)| !matches!(control, Control::Video | Control::Gif))
    );
    crate::app::actions::run(&mut state, crate::keys::Action::RecordGif, &mut 0);
    assert_eq!(state.capture_start.recording, None);
    assert_eq!(state.mode, Mode::Editor);
    click(&mut state, Control::Fullscreen);
    assert!(state.selection.zone.is_some());
}

/// 【截图起始测试】【录制快捷键】快捷键与顶部按钮进入相同的录制选区模式，选区形成后不再响应
#[test]
fn recording_shortcuts_match_the_top_buttons() {
    use crate::keys::Action;
    let mut state = editor();
    for (action, format) in [
        (Action::RecordVideo, Format::Video),
        (Action::RecordGif, Format::Gif),
    ] {
        crate::app::actions::run(&mut state, action, &mut 0);
        assert_eq!(state.capture_start.recording, Some(format));
        assert_eq!(state.mode, Mode::Region);
    }
    click(&mut state, Control::Region);
    click(&mut state, Control::Fullscreen);
    assert!(!state.capture_start.visible);
    crate::app::actions::run(&mut state, Action::RecordVideo, &mut 0);
    assert!(state.capture_start.recording.is_none());
}

/// 【截图起始测试】【无效拖选】不足八像素的拖动不构成有效选区，起始入口继续可用
#[test]
fn rejected_tiny_selection_preserves_start_controls() {
    let mut state = editor();
    crate::app::input::handle_pointer_move(&mut state, 0, 80.0, 120.0, &mut 0);
    crate::app::input::handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    crate::app::input::handle_pointer_move(&mut state, 0, 83.0, 123.0, &mut 0);
    sync(&mut state, &mut 0);
    crate::app::input::handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    sync(&mut state, &mut 0);
    assert!(state.capture_start.visible && state.selection.zone.is_none());
}

/// 【截图起始测试】【空闲窗口】已有录制控制窗口时不启动第二个窗口，避免选区交接后发生独占锁冲突
#[test]
fn idle_recording_window_prevents_duplicate_launch() {
    let mut state = editor();
    state.capture_start.status.window_open = true;
    click(&mut state, Control::Video);
    assert_eq!(state.capture_start.recording, None);
    assert_eq!(state.mode, Mode::Editor);
}

/// 【截图起始测试】【格式兼容】旧命令行目标仍能反序列化，显式 GIF 入口不受默认视频容器影响
#[test]
fn recording_target_is_backward_compatible_and_format_specific() {
    let target: crate::features::recording::Target =
        serde_json::from_str(r#"{"display":"DP-1","crop":null}"#).unwrap();
    assert!(target.format.is_none());
    assert_eq!(Format::Gif.resolve("mkv"), "gif");
    assert_eq!(Format::Video.resolve("gif"), "mp4");
    assert_eq!(Format::Video.resolve("mkv"), "mkv");
}
