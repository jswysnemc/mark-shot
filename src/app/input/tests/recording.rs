//! 【录屏工具测试】【交互链路】验证普通工具、区域保留、小工具栏菜单与开始请求

use super::*;
use crate::{
    config::recording::Recording,
    features::recording::tool_options::Parameter,
    keys::{Chord, Key},
    tools::Tool,
    types::{Mode, MouseButton},
    ui::settings_panel::{SettingsAction, SettingsSource, SettingsWidget},
};

/// 【录屏工具测试】【场景构造】无参数，返回允许录屏且后台空闲的截图编辑器
fn recording() -> EditorState {
    let mut state = editor();
    crate::app::capture_start::initialize(&mut state, &crate::app::Launch::default());
    state.capture_start.checked = true;
    state.recording_tool.config = Recording::default();
    super::super::select_tool(&mut state, Tool::Recording, &mut 0);
    state
}

/// 【录屏工具测试】【指针位置】按 x、y 更新 state 实际输入坐标，不模拟控件回调
fn pointer(state: &mut EditorState, x: f64, y: f64) {
    super::super::handle_pointer_move(state, 0, x, y, &mut 0);
}

/// 【录屏工具测试】【区域绘制】按 state 发送完整区域拖选，返回前处理正式释放
fn region(state: &mut EditorState) {
    pointer(state, 80.0, 140.0);
    super::super::handle_pointer_button(state, MouseButton::Left, true, &mut 0);
    pointer(state, 360.0, 280.0);
    super::super::handle_pointer_button(state, MouseButton::Left, false, &mut 0);
    crate::app::capture_start::sync(state, &mut 0);
}

/// 【录屏工具测试】【属性点击】按 action 在 state 对应小工具栏控件发送完整鼠标点击
fn click(state: &mut EditorState, action: SettingsAction) {
    let index = state
        .settings_panel
        .widgets
        .iter()
        .position(|widget| {
            matches!(widget,
                SettingsWidget::Action { action: candidate, .. } if *candidate == action
            )
        })
        .unwrap();
    let rect = state.settings_panel.widget_rect(index).unwrap();
    pointer(
        state,
        f64::from(rect.left() + rect.width() / 2.0),
        f64::from(rect.top() + rect.height() / 2.0),
    );
    for pressed in [true, false] {
        super::super::handle_pointer_button(state, MouseButton::Left, pressed, &mut 0);
    }
}

/// 【录屏工具测试】【菜单按键】按 key 经完整快捷键入口操作 state 菜单，不直接写参数
fn key(state: &mut EditorState, key: Key) {
    super::super::handle_key(
        state,
        Some(Chord {
            key,
            mods: Default::default(),
        }),
        None,
        &mut 0,
    );
}

/// 【录屏工具测试】【区域保留】框选完成仍选中录屏，工具栏可调整参数，未创建标注或开始请求
#[test]
fn region_selection_keeps_the_recording_tool_and_editor_open() {
    let mut state = recording();
    state.after_selection_tool = Tool::Rectangle;
    assert!(state.selection.zone.is_none());
    region(&mut state);
    assert_eq!(state.mode, Mode::Editor);
    assert_eq!(state.selected_tool, Tool::Recording);
    assert!(!state.ui_hidden && state.settings_panel.visible);
    assert_eq!(
        state.settings_panel.active_source,
        Some(SettingsSource::Tool(Tool::Recording))
    );
    assert!(state.finish.is_none() && state.recording_tool.request.is_none());
    assert!(state.annotations.is_empty() && state.undo_stack.is_empty());
    let old = state.selection.zone.unwrap();
    pointer(&mut state, 160.0, 200.0);
    super::super::handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 180.0, 210.0);
    super::super::handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    assert_ne!(state.selection.zone, Some(old));
    assert_eq!(state.selected_tool, Tool::Recording);
    assert!(state.finish.is_none());
}

/// 【录屏工具测试】【菜单提交】格式菜单消费 Enter，GIF 显示独立帧率，参数修改不污染标注撤销历史
#[test]
fn toolbar_menu_changes_format_without_exporting_or_adding_history() {
    let mut state = recording();
    region(&mut state);
    let zone = state.selection.zone;
    click(&mut state, SettingsAction::Recording(Parameter::Format));
    assert!(state.recording_tool.menu.is_some());
    key(&mut state, Key::End);
    state.damage_rects.clear();
    key(&mut state, Key::Enter);
    let panel = state.settings_panel.rect().unwrap();
    assert!(state.damage_rects.iter().any(|zone| matches!(zone,
        crate::editor::DamageZone::Local { monitor_idx: 0, rect } if *rect == panel
    )));
    assert_eq!(state.recording_tool.config.format, "gif");
    assert_eq!(state.settings_panel.recording.frame_rate(), 12);
    assert!(state.recording_tool.menu.is_none() && state.finish.is_none());
    assert_eq!(state.selection.zone, zone);
    assert!(state.undo_stack.is_empty());
    click(&mut state, SettingsAction::Recording(Parameter::Audio));
    assert!(state.recording_tool.menu.is_none());
    click(&mut state, SettingsAction::Recording(Parameter::Cursor));
    key(&mut state, Key::Home);
    key(&mut state, Key::Enter);
    assert!(!state.recording_tool.config.cursor);
}

/// 【录屏工具测试】【点击归属】菜单外部完整点击仅关闭菜单，右键不清除区域，Escape 不退出截图
#[test]
fn popup_outside_click_and_escape_do_not_reach_the_region() {
    let mut state = recording();
    region(&mut state);
    let zone = state.selection.zone;
    click(&mut state, SettingsAction::Recording(Parameter::Quality));
    key(&mut state, Key::Escape);
    assert!(state.recording_tool.menu.is_none() && !state.cancel);
    for button in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
        click(&mut state, SettingsAction::Recording(Parameter::Quality));
        pointer(&mut state, 2.0, 2.0);
        for pressed in [true, false] {
            super::super::handle_pointer_button(&mut state, button, pressed, &mut 0);
        }
        assert!(state.recording_tool.menu.is_none());
        assert_eq!(state.selection.zone, zone);
        assert!(!state.input.mouse_down && !state.tool_active);
    }
}

/// 【录屏工具测试】【明确开始】小工具栏开始按钮提交参数快照；缺少区域或后台占用时保留编辑界面
#[test]
fn start_requires_a_valid_region_and_idle_session_then_snapshots_options() {
    let mut state = recording();
    crate::app::panels::recording::start(&mut state, &mut 0);
    assert!(state.recording_tool.request.is_none() && state.finish.is_none());
    region(&mut state);
    state.capture_start.status.window_open = true;
    click(&mut state, SettingsAction::StartRecording);
    assert!(state.recording_tool.request.is_none() && state.finish.is_none());
    state.capture_start.status.window_open = false;
    state.recording_tool.config.fps = 24;
    state.recording_tool.config.countdown = 0;
    super::super::refresh_panels(&mut state, &mut 0);
    click(&mut state, SettingsAction::StartRecording);
    assert!(state.finish.is_some());
    let config = state.recording_tool.request.as_ref().unwrap().clone();
    assert_eq!((config.fps, config.countdown), (24, 0));
    state.recording_tool.config.fps = 60;
    assert_eq!(state.recording_tool.request.unwrap(), config);
}

/// 【录屏工具测试】【场景隔离】专用识别模式不能选择录屏，也不显示录屏按钮
#[test]
fn dedicated_modes_do_not_expose_the_recording_tool() {
    use crate::ui::toolbar::{ToolbarButton, ToolbarItem};
    let mut state = editor();
    crate::app::capture_start::initialize(
        &mut state,
        &crate::app::Launch {
            mode: Mode::Ocr,
            ..Default::default()
        },
    );
    super::super::select_tool(&mut state, Tool::Recording, &mut 0);
    assert_eq!(state.selected_tool, Tool::Selection);
    super::super::refresh_panels(&mut state, &mut 0);
    assert!(
        !state
            .toolbar
            .items
            .contains(&ToolbarItem::Button(ToolbarButton::Tool(Tool::Recording)))
    );
}

/// 【录屏工具测试】【数值范围】视频与 GIF 保留各自帧率，滚轮微调和非法选项都遵守合法边界
#[test]
fn format_parameters_keep_independent_frame_rates_and_valid_limits() {
    let mut config = Recording {
        fps: 50,
        gif_fps: 17,
        countdown: 7,
        audio: "system".into(),
        ..Default::default()
    };
    Parameter::Format.choose(2, &mut config);
    assert_eq!(config.frame_rate(), 17);
    Parameter::Fps.step(100, &mut config);
    assert_eq!(config.gif_fps, 30);
    Parameter::Audio.choose(0, &mut config);
    assert_eq!(config.audio, "system");
    Parameter::Format.choose(0, &mut config);
    assert_eq!(config.frame_rate(), 50);
    Parameter::Fps.step(-100, &mut config);
    assert_eq!(config.fps, 1);
    Parameter::Countdown.step(100, &mut config);
    assert_eq!(config.countdown, 10);
    assert!(config.validate().is_ok());
}

/// 【录屏工具测试】【区域入口参数】重新框选保留调用方传入的 GIF 参数，Enter 只提交录制请求
#[test]
fn region_launch_keeps_provided_options_and_accept_starts_recording() {
    let mut state = editor();
    let config = Recording {
        format: "gif".into(),
        gif_fps: 17,
        countdown: 5,
        ..Default::default()
    };
    crate::app::capture_start::initialize(
        &mut state,
        &crate::app::Launch {
            record_select: true,
            record_config: Some(config.clone()),
            ..Default::default()
        },
    );
    assert_eq!(state.recording_tool.config, config);
    assert_eq!(
        state.capture_start.recording,
        Some(crate::features::recording::Format::Gif)
    );
    state.capture_start.checked = true;
    super::super::select_tool(&mut state, Tool::Recording, &mut 0);
    region(&mut state);
    key(&mut state, Key::Enter);
    assert_eq!(state.recording_tool.request, Some(config));
    assert!(state.finish.is_some());
}
