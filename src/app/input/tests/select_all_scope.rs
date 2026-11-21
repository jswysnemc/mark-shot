//! 【全图快捷键测试】【输入一致性】验证 Ctrl+A 与中键共享全图范围，保留选区恢复和文字输入职责

use super::{editor, key, pointer};
use crate::{
    app::input::{handle_pointer_button, select_tool},
    editor::{EditorState, Layers},
    tools::Tool,
    types::{MouseButton, Placement},
    ui::panel::UiPanel,
};
use tiny_skia::Rect;

/// 【全图快捷键测试】【多屏准备】无参数，返回具有左侧副屏的编辑器，不连接显示服务器
fn two_monitors() -> EditorState {
    // 1. 【全图快捷键测试】【图层准备】复用单屏样本图层和字体状态
    let original = editor();
    let primary = original.placements[0].clone();
    // 2. 【全图快捷键测试】【工作区创建】建立两个相邻显示器及对应图层
    EditorState::new(
        Layers {
            captures: vec![original.captures[0].clone(); 2],
            canvas: vec![original.canvas[0].clone(); 2],
            dimmed: vec![original.dimmed[0].clone(); 2],
            annotations: vec![original.annotations_layer[0].clone(); 2],
        },
        vec![
            primary.clone(),
            Placement {
                position: (-640, 0),
                ..primary
            },
        ],
        original.icons_cache,
        original.text,
        original.ocr,
    )
}

/// 【全图快捷键测试】【中键对照】无参数和返回值，真实 Ctrl+A 与中键均进入跨屏全图，并可恢复无选区状态
#[test]
fn ctrl_a_matches_middle_click_and_returns_to_unselected_state() {
    // 1. 【全图快捷键测试】【同源输入】分别发送真实按键和完整中键点击
    let mut keyboard = two_monitors();
    let mut mouse = two_monitors();
    pointer(&mut keyboard, 300.0, 230.0, &mut 0);
    pointer(&mut mouse, 300.0, 230.0, &mut 0);
    key(&mut keyboard, "Ctrl+A", &mut 0);
    for pressed in [true, false] {
        handle_pointer_button(&mut mouse, MouseButton::Middle, pressed, &mut 0);
    }
    // 2. 【全图快捷键测试】【全图状态】两种入口必须覆盖完整工作区，并进入固定全图范围
    assert_eq!(keyboard.selection.zone, mouse.selection.zone);
    assert_eq!(
        keyboard.selection.zone,
        Rect::from_xywh(-640.0, 0.0, 1280.0, 480.0)
    );
    assert!(keyboard.fullscreen_scope && mouse.fullscreen_scope);
    assert_eq!(keyboard.selected_tool, mouse.selected_tool);
    // 3. 【全图快捷键测试】【选择恢复】再次按键恢复选择阶段，隐藏没有选区的工具栏
    key(&mut keyboard, "Ctrl+A", &mut 0);
    assert!(!keyboard.fullscreen_scope && keyboard.selection.zone.is_none());
    assert!(keyboard.toolbar.rect().is_none());
}

/// 【全图快捷键测试】【已有选区】无参数和返回值，切换全图后恢复原选区，保留已完成标注
#[test]
fn ctrl_a_restores_original_region_without_changing_annotations() {
    // 1. 【全图快捷键测试】【标注准备】建立部分选区及已完成文字，离开文字输入状态
    let mut state = editor();
    let original = Rect::from_xywh(70.0, 80.0, 300.0, 200.0);
    state.selection.zone = original;
    super::text::add_text(&mut state);
    select_tool(&mut state, Tool::Pick, &mut 0);
    let annotations = state.annotations.clone();
    // 2. 【全图快捷键测试】【范围切换】进入全图时记录原选区并回到选择工具
    key(&mut state, "Ctrl+A", &mut 0);
    assert!(state.fullscreen_scope);
    assert_eq!(state.scope_before_fullscreen, original);
    assert_eq!(state.selected_tool, Tool::Selection);
    // 3. 【全图快捷键测试】【原样恢复】再次按键恢复原选区，标注内容保持不变
    key(&mut state, "Ctrl+A", &mut 0);
    assert!(!state.fullscreen_scope);
    assert_eq!(state.selection.zone, original);
    assert_eq!(state.annotations, annotations);
}

/// 【全图快捷键测试】【拖动保护】无参数和返回值，选择拖动期间 Ctrl+A 不改变临时选区或进入全图
#[test]
fn ctrl_a_does_not_replace_an_active_selection_drag() {
    // 1. 【全图快捷键测试】【拖动准备】使用真实鼠标输入建立未释放的临时选区
    let mut state = editor();
    pointer(&mut state, 100.0, 100.0, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 220.0, 210.0, &mut 0);
    let original = state.selection.zone;
    // 2. 【全图快捷键测试】【快捷保护】按键保留拖动状态，释放后仍得到原选区
    key(&mut state, "Ctrl+A", &mut 0);
    assert_eq!(state.selection.zone, original);
    assert!(!state.fullscreen_scope);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    assert_eq!(state.selection.zone, original);
}

/// 【全图快捷键测试】【识别全选】无参数和返回值，OCR 内 Ctrl+A 继续选择识别文字，不切换截图范围
#[test]
fn ctrl_a_keeps_ocr_text_selection_separate_from_capture_scope() {
    // 1. 【全图快捷键测试】【识别准备】直接提供已有识别结果，不启动识别进程或读取模型
    let mut state = editor();
    state.selected_tool = Tool::Ocr;
    let region = Rect::from_xywh(80.0, 90.0, 300.0, 200.0).unwrap();
    state.selection.zone = Some(region);
    state.ocr.view.set_region(region);
    state.ocr.view.set_lines(vec![crate::ocr::OcrLine {
        text: "alpha".into(),
        bounds: Rect::from_xywh(100.0, 110.0, 50.0, 20.0).unwrap(),
        char_x: vec![100.0, 110.0, 120.0, 130.0, 140.0, 150.0],
    }]);
    assert!(!state.ocr.view.has_selection());
    // 2. 【全图快捷键测试】【识别输入】真实快捷键选择全文并保留识别工具与截图选区
    key(&mut state, "Ctrl+A", &mut 0);
    assert!(state.ocr.view.has_selection());
    assert_eq!(state.ocr.view.text_to_copy(), "alpha");
    assert_eq!(state.selected_tool, Tool::Ocr);
    assert_eq!(state.selection.zone, Some(region));
    assert!(!state.fullscreen_scope);
}
