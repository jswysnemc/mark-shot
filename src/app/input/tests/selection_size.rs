//! 【选区尺寸测试】【实际输入】通过应用输入分派核对编辑归属、比例拖动、拒绝尺寸与真实 PNG 导出

use super::*;
use crate::{
    app::selection_size,
    features::selection_size::ratio::Field,
    types::{MouseButton, ScrollAxis, ScrollEvent},
    ui::selection_size::state::Hit,
};
use tiny_skia::Rect;

/// 【选区尺寸测试】【正常选区】返回具有可见尺寸面板的编辑器，不连接显示服务器
pub(super) fn selected() -> EditorState {
    let mut state = editor();
    state.selection.zone = Rect::from_xywh(100.0, 100.0, 300.0, 180.0);
    super::super::refresh_panels(&mut state, &mut 0);
    assert!(state.selection_size.panel.as_ref().unwrap().rect.is_some());
    state
}

/// 【选区尺寸测试】【面板坐标】按 state 和 hit 返回当前控件中心，背景使用左侧内边距
fn center(state: &EditorState, hit: Hit) -> (f64, f64) {
    let panel = state.selection_size.panel.as_ref().unwrap();
    let bounds = panel.rect.unwrap();
    let local = match hit {
        Hit::Field(side) => panel.layout.field(side),
        Hit::Lock => panel.layout.lock,
        Hit::Ratio => panel.layout.ratio,
        Hit::Background => Rect::from_xywh(1.0, 5.0, 3.0, 16.0).unwrap(),
    };
    (
        f64::from(bounds.left() + local.left() + local.width() / 2.0),
        f64::from(bounds.top() + local.top() + local.height() / 2.0),
    )
}

/// 【选区尺寸测试】【控件按钮】按 hit、button 和 state 发送完整按下释放，返回空值
fn click(state: &mut EditorState, hit: Hit, button: MouseButton) {
    let point = center(state, hit);
    pointer(state, point.0, point.1, &mut 0);
    for pressed in [true, false] {
        super::super::handle_pointer_button(state, button, pressed, &mut 0);
    }
}

/// 【选区尺寸测试】【整批文字】按 text 通过应用入口输入 state，返回空值
fn text(state: &mut EditorState, text: &str) {
    super::super::handle_key(state, None, Some(text.into()), &mut 0);
}

/// 【选区尺寸测试】【确认取消归属】键盘待提交值不改变选区，Enter 应用，Escape 只结束字段并保留截图窗口
#[test]
fn arrows_wait_for_commit_and_escape_belongs_to_the_field() {
    let mut state = selected();
    key(&mut state, "Ctrl+R", &mut 0);
    text(&mut state, "220");
    key(&mut state, "Up", &mut 0);
    assert_eq!(state.selection.zone.unwrap().width(), 300.0);
    assert_eq!(
        state.selection_size.panel.as_ref().unwrap().fields[0]
            .editor
            .text,
        "221"
    );
    key(&mut state, "Return", &mut 0);
    assert_eq!(state.selection.zone.unwrap().width(), 221.0);
    assert!(!state.selection_size.interacting());
    assert!(state.finish.is_none());
    key(&mut state, "Ctrl+R", &mut 0);
    text(&mut state, "180");
    key(&mut state, "Escape", &mut 0);
    assert!(!state.cancel);
    assert_eq!(state.selection.zone.unwrap().width(), 221.0);
    assert!(state.selection_size.caret.deadline().is_none());
}

/// 【选区尺寸测试】【宽度快捷键】高度字段已有待提交文字时，精确 Ctrl+R 提交高度并切回宽度
#[test]
fn ctrl_r_from_height_commits_then_selects_width() {
    let mut state = selected();
    key(&mut state, "Ctrl+R", &mut 0);
    key(&mut state, "Tab", &mut 0);
    text(&mut state, "96");
    key(&mut state, "Ctrl+R", &mut 0);
    assert_eq!(
        state.selection_size.panel.as_ref().unwrap().editing(),
        Some(Field::Width)
    );
    assert_eq!(state.selection.zone.unwrap().height(), 96.0);
    assert_eq!(
        state.selection_size.panel.as_ref().unwrap().fields[0]
            .editor
            .selected_text()
            .as_deref(),
        Some("300")
    );
}

/// 【选区尺寸测试】【词语按键】原版 Ctrl+Left 跨越整个数字段，额外 Alt 修饰不执行字符移动
#[test]
fn word_navigation_and_alt_modifiers_match_the_field() {
    let mut state = selected();
    key(&mut state, "Ctrl+R", &mut 0);
    text(&mut state, "220");
    key(&mut state, "Ctrl+Left", &mut 0);
    assert_eq!(
        state.selection_size.panel.as_ref().unwrap().fields[0]
            .editor
            .cursor,
        0
    );
    key(&mut state, "Ctrl+Right", &mut 0);
    assert_eq!(
        state.selection_size.panel.as_ref().unwrap().fields[0]
            .editor
            .cursor,
        3
    );
    key(&mut state, "Alt+Left", &mut 0);
    assert_eq!(
        state.selection_size.panel.as_ref().unwrap().fields[0]
            .editor
            .cursor,
        3
    );
    assert_eq!(state.selection.zone.unwrap().width(), 300.0);
}

/// 【选区尺寸测试】【标准快捷键】Ctrl+Home 与 Ctrl+Shift+A 在原版字段中没有导航或全选动作
#[test]
fn extra_shortcut_modifiers_do_not_select_or_move_the_field() {
    let mut state = selected();
    key(&mut state, "Ctrl+R", &mut 0);
    text(&mut state, "220");
    key(&mut state, "Ctrl+Home", &mut 0);
    assert_eq!(
        state.selection_size.panel.as_ref().unwrap().fields[0]
            .editor
            .cursor,
        3
    );
    key(&mut state, "Ctrl+Shift+A", &mut 0);
    assert!(
        state.selection_size.panel.as_ref().unwrap().fields[0]
            .editor
            .selection_range()
            .is_none()
    );
}

/// 【选区尺寸测试】【无焦点按钮】右键字段和锁没有动作，左键锁使用实际尺寸且保留用户待提交文字
#[test]
fn lock_and_background_preserve_pending_text_and_ignore_right_button() {
    let mut state = selected();
    click(&mut state, Hit::Field(Field::Width), MouseButton::Right);
    assert!(!state.selection_size.interacting());
    click(&mut state, Hit::Field(Field::Width), MouseButton::Left);
    text(&mut state, "240");
    click(&mut state, Hit::Lock, MouseButton::Right);
    assert_eq!(state.selection.aspect_ratio, 0.0);
    click(&mut state, Hit::Background, MouseButton::Left);
    click(&mut state, Hit::Lock, MouseButton::Left);
    assert_eq!(state.selection.aspect_ratio, 300.0 / 180.0);
    assert_eq!(
        state.selection_size.panel.as_ref().unwrap().fields[0]
            .editor
            .text,
        "240"
    );
    assert!(state.selection_size.panel.as_ref().unwrap().fields[0].editing);
    key(&mut state, "Return", &mut 0);
    assert_eq!(state.selection.zone.unwrap().width(), 240.0);
    assert_eq!(state.selection.zone.unwrap().height(), 144.0);
}

/// 【选区尺寸测试】【横向滚轮】编辑空文本时回退实际尺寸，Shift 横轴立即调整十像素并保留字段焦点
#[test]
fn wheel_uses_actual_value_for_empty_text_and_supports_shift_horizontal_axis() {
    let mut state = selected();
    click(&mut state, Hit::Field(Field::Width), MouseButton::Left);
    key(&mut state, "Backspace", &mut 0);
    super::super::handle_modifiers(&mut state, false, true, &mut 0);
    super::super::handle_axis_scroll(
        &mut state,
        ScrollEvent {
            horizontal: ScrollAxis {
                value120: -120,
                ..Default::default()
            },
            ..Default::default()
        },
        &mut 0,
    );
    assert_eq!(state.selection.zone.unwrap().width(), 290.0);
    let field = &state.selection_size.panel.as_ref().unwrap().fields[0];
    assert!(field.editing);
    assert_eq!(field.editor.selected_text().as_deref(), Some("290"));
    key(&mut state, "Escape", &mut 0);
    assert_eq!(state.selection.zone.unwrap().width(), 290.0);
}

/// 【选区尺寸测试】【比例选择应用】打开列表取消字段，右键释放无动作，选择一比一立即保留实际宽度并联动高度
#[test]
fn choosing_ratio_cancels_draft_then_resizes_the_real_selection() {
    let mut state = selected();
    click(&mut state, Hit::Field(Field::Width), MouseButton::Left);
    text(&mut state, "240");
    click(&mut state, Hit::Ratio, MouseButton::Left);
    assert!(
        state
            .selection_size
            .panel
            .as_ref()
            .unwrap()
            .editing()
            .is_none()
    );
    let popup = state.selection_size.popup.as_ref().unwrap();
    let id = popup.id;
    for button in [MouseButton::Right, MouseButton::Left] {
        selection_size::popup::event(
            &mut state,
            id,
            crate::types::popup::PopupEvent::Button {
                x: 20.0,
                y: 43.0,
                button,
                pressed: false,
            },
            &mut 0,
        );
        if matches!(button, MouseButton::Right) {
            assert!(state.selection_size.popup.is_some());
        }
    }
    assert!(state.selection_size.popup.is_none());
    assert_eq!(state.selection.aspect_ratio, 1.0);
    assert_eq!(state.selection.zone.unwrap().width(), 300.0);
    assert_eq!(state.selection.zone.unwrap().height(), 300.0);
    assert!(!state.input.mouse_down);
    assert!(state.pending.is_none());
}

/// 【选区尺寸测试】【正方形修饰】创建拖动中按下及松开 Shift 立即切换形状，锁定比例优先于修饰键
#[test]
fn shift_changes_an_existing_creation_drag_and_locked_ratio_takes_priority() {
    let mut state = editor();
    pointer(&mut state, 100.0, 100.0, &mut 0);
    super::super::handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 250.0, 190.0, &mut 0);
    assert_eq!(state.selection.zone.unwrap().height(), 90.0);
    super::super::handle_modifiers(&mut state, false, true, &mut 0);
    assert_eq!(state.selection.zone.unwrap().height(), 150.0);
    super::super::handle_modifiers(&mut state, false, false, &mut 0);
    assert_eq!(state.selection.zone.unwrap().height(), 90.0);
    state.selection.aspect_ratio = 2.0;
    super::super::handle_modifiers(&mut state, false, true, &mut 0);
    assert_eq!(state.selection.zone.unwrap().width(), 180.0);
    assert_eq!(state.selection.zone.unwrap().height(), 90.0);
}

/// 【选区尺寸测试】【分数指针创建】自由拖动保留原版分数端点，不在物理导出前扩大逻辑矩形
#[test]
fn free_creation_keeps_fractional_pointer_endpoints() {
    let mut state = editor();
    pointer(&mut state, 100.25, 100.5, &mut 0);
    super::super::handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 250.75, 190.75, &mut 0);
    let selection = state.selection.zone.unwrap();
    assert_eq!(
        selection,
        Rect::from_xywh(100.25, 100.5, 150.5, 90.25).unwrap()
    );
}

/// 【选区尺寸测试】【无效创建释放】不足八像素的初次范围在释放时清除，不自动进入默认标注工具或快捷导出
#[test]
fn tiny_creation_returns_to_selecting_without_switching_default_tool() {
    let mut state = editor();
    state.after_selection_tool = crate::tools::Tool::Marker;
    pointer(&mut state, 100.0, 100.0, &mut 0);
    super::super::handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 107.0, 110.0, &mut 0);
    assert!(state.selection.zone.is_some());
    super::super::handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    assert!(state.selection.zone.is_none());
    assert_eq!(state.selected_tool, crate::tools::Tool::Selection);
    assert!(state.finish.is_none());
    assert!(!state.selection.creating);
    assert!(!state.input.mouse_down);
}

/// 【选区尺寸测试】【最小尺寸拒绝】小于八像素的输入触发一千四百毫秒提示，实际选区和历史保持原值
#[test]
fn too_small_input_is_rejected_with_the_original_toast_lifetime() {
    let mut state = selected();
    let original = state.selection.zone;
    click(&mut state, Hit::Field(Field::Width), MouseButton::Left);
    text(&mut state, "7");
    key(&mut state, "Return", &mut 0);
    assert_eq!(state.selection.zone, original);
    assert!(state.undo_stack.is_empty());
    let toast = state
        .toasts
        .items
        .iter()
        .find(|toast| toast.kind == crate::ui::toast::ToastKind::SelectionTooSmall)
        .unwrap();
    assert!(
        matches!(toast.spec.life, crate::ui::toast::ToastLife::Timed(duration) if duration == std::time::Duration::from_millis(1400))
    );
}

/// 【选区尺寸测试】【真实物理导出】一倍、一点五倍和两倍截图都以物理像素调整，最终 PNG 不额外扩大逻辑边缘
#[test]
fn exact_pixel_size_survives_png_export_at_fractional_output_scale() {
    for scale in [1.0, 1.5, 2.0] {
        let mut state = selected();
        state.placements[0].scale = scale;
        state.captures[0].scale = scale;
        state.captures[0].pixmap = Pixmap::new((640.0 * scale) as u32, (480.0 * scale) as u32)
            .unwrap()
            .into();
        state.captures[0]
            .pixmap
            .fill(Color::from_rgba8(30, 34, 38, 255));
        state.selection.zone = Rect::from_xywh(20.0, 30.0, 200.0, 100.0);
        assert!(selection_size::apply::size(&mut state, (100, 80), &mut 0));
        let png = crate::app::render_final(&mut state).unwrap().0;
        let image = Pixmap::decode_png(&png).unwrap();
        assert_eq!((image.width(), image.height()), (100, 80), "scale={scale}");
    }
    let mut state = selected();
    state.placements[0].position = (101, 53);
    state.placements[0].scale = 1.5;
    state.captures[0].scale = 1.5;
    state.captures[0].pixmap = Pixmap::new(960, 720).unwrap().into();
    state.selection.zone = Rect::from_xywh(101.0, 53.0, 200.0, 100.0);
    assert!(selection_size::apply::size(&mut state, (100, 80), &mut 0));
    let pixels = Pixmap::decode_png(&crate::app::render_final(&mut state).unwrap().0).unwrap();
    assert_eq!((pixels.width(), pixels.height()), (100, 80));
}

/// 【选区尺寸测试】【原生输入拥有者】字段声明普通文字用途，候选不调整尺寸，程序滚轮替换后拒绝旧确认身份
#[test]
fn native_field_context_is_normal_text_and_stale_commits_are_rejected() {
    use crate::types::{
        OverlayEvent,
        text_input::{Batch, ContentType, Focus, Preedit, Update},
    };
    let mut state = selected();
    state.input.keyboard_focus = Some(0);
    key(&mut state, "Ctrl+R", &mut 0);
    crate::app::text_input::event(
        &mut state,
        OverlayEvent::TextInputFocus(Focus {
            id: 9100,
            monitor_idx: 0,
        }),
        &mut 0,
    );
    let first = super::native_field_support::sync(&mut state)
        .pop()
        .unwrap()
        .unwrap();
    assert_eq!(first.content, ContentType::Normal);
    assert_eq!(first.surrounding.as_ref().unwrap().text, "300");
    super::native_field_support::deliver(
        &mut state,
        Batch {
            preedit: Preedit {
                text: "候选".into(),
                cursor: Some((6, 6)),
            },
            ..Default::default()
        },
        true,
    );
    assert_eq!(state.selection.zone.unwrap().width(), 300.0);
    let next = super::native_field_support::sync(&mut state)
        .pop()
        .unwrap()
        .unwrap();
    assert_eq!(next.surrounding.as_ref().unwrap().text, "");
    assert_ne!(first.cursor_rect, next.cursor_rect);
    assert!(super::native_field_support::sync(&mut state).is_empty());
    let point = center(&state, Hit::Field(Field::Width));
    pointer(&mut state, point.0, point.1, &mut 0);
    super::super::handle_axis_scroll(&mut state, ScrollEvent::wheel(-120), &mut 0);
    crate::app::text_input::event(
        &mut state,
        OverlayEvent::TextInput(Update {
            target_id: first.target_id,
            serial: 2,
            sync_allowed: true,
            batch: Batch {
                commit: "999".into(),
                ..Default::default()
            },
        }),
        &mut 0,
    );
    assert_eq!(
        state.selection_size.panel.as_ref().unwrap().fields[0]
            .editor
            .text,
        "299"
    );
    assert_eq!(state.selection.zone.unwrap().width(), 299.0);
    let after = super::native_field_support::sync(&mut state)
        .pop()
        .unwrap()
        .unwrap();
    assert_ne!(first.target_id, after.target_id);
    assert_eq!(after.content, ContentType::Normal);
}

/// 【选区尺寸测试】【原图物理导出】缩小原图视口中的对齐选区按整数物理尺寸导出，真实分数原点仍按原版扩展边缘
#[test]
fn exact_file_size_and_fractional_native_origins_use_image_space_alignment() {
    let mut state = selected();
    let file = crate::editor::file_canvas::FileCanvas::new(
        Pixmap::new(1200, 600).unwrap(),
        Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap(),
    )
    .unwrap();
    let space_scale = file.original.width() as f32 / file.viewport.width();
    state.selection.zone = Rect::from_xywh(
        file.viewport.left() + 30.0 / space_scale,
        file.viewport.top() + 30.0 / space_scale,
        200.0 / space_scale,
        120.0 / space_scale,
    );
    state.file_canvas = Some(file);
    assert!(selection_size::apply::size(&mut state, (100, 80), &mut 0));
    let png = crate::app::render_final(&mut state).unwrap().0;
    let image = Pixmap::decode_png(&png).unwrap();
    assert_eq!((image.width(), image.height()), (100, 80));
    let mut state = selected();
    state.captures[0].scale = 1.5;
    state.placements[0].scale = 1.5;
    state.captures[0].pixmap = Pixmap::new(960, 720).unwrap().into();
    state.selection.zone = Rect::from_xywh(20.125, 30.125, 200.0, 100.0);
    assert!(selection_size::apply::size(&mut state, (100, 80), &mut 0));
    let png = crate::app::render_final(&mut state).unwrap().0;
    let image = Pixmap::decode_png(&png).unwrap();
    assert_eq!((image.width(), image.height()), (101, 81));
}

/// 【选区尺寸测试】【资格与层级】不足八像素的原始范围不显示字段，回退重叠时工具栏优先消费点击
#[test]
fn visibility_uses_unrounded_pixels_and_fallback_preserves_toolbar_hit_priority() {
    let mut state = selected();
    state.selection.zone = Rect::from_xywh(100.0, 100.0, 7.6, 80.0);
    super::super::refresh_panels(&mut state, &mut 0);
    assert!(state.selection_size.panel.as_ref().unwrap().rect.is_none());
    let mut state = selected();
    let toolbar = state.toolbar.rect().unwrap();
    let panel = state.selection_size.panel.as_mut().unwrap();
    panel.rect = Rect::from_xywh(
        toolbar.left(),
        toolbar.top(),
        panel.layout.size.0,
        panel.layout.size.1,
    );
    panel.under_toolbars = true;
    let point = center(&state, Hit::Field(Field::Width));
    pointer(&mut state, point.0, point.1, &mut 0);
    assert!(selection_size::input::hit(&state).is_none());
    super::super::handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    assert!(
        state
            .selection_size
            .panel
            .as_ref()
            .unwrap()
            .editing()
            .is_none()
    );
    assert!(!state.input.mouse_down);
}

/// 【选区尺寸测试】【拖动拥有者】范围创建时隐藏，普通标注拖动继续显示；画布正在拖动时字段不能中断手势
#[test]
fn selection_creation_visibility_is_distinct_from_annotation_drag_and_mouse_grab() {
    let mut state = selected();
    state.selected_tool = crate::tools::Tool::Marker;
    state.input.mouse_down = true;
    state.input.drag_start = Some((100.0, 100.0));
    selection_size::panels::update(&mut state, &mut 0);
    assert!(state.selection_size.panel.as_ref().unwrap().rect.is_some());
    let point = center(&state, Hit::Field(Field::Width));
    state.input.pointer.local = point;
    assert!(selection_size::input::hit(&state).is_none());
    assert!(!selection_size::input::pointer(&mut state, &mut 0));
    state.selection.creating = true;
    selection_size::panels::update(&mut state, &mut 0);
    assert!(state.selection_size.panel.as_ref().unwrap().rect.is_none());
}

/// 【选区尺寸测试】【原版裁切参考】九个实际 Qt 对齐样本覆盖标准比例与捕获缓冲比例，最终 PNG 严格匹配原版宽高
#[test]
fn physical_png_bounds_match_real_qt_alignment_after_exact_resize() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-selection-size/reference.json"
    ))
    .unwrap();
    for sample in reference["aligned_exports"].as_array().unwrap() {
        let scale = sample["scale"].as_f64().unwrap() as f32;
        let values = sample["current"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_f64().unwrap() as f32 / scale)
            .collect::<Vec<_>>();
        let mut state = selected();
        state.captures[0].scale = scale;
        state.placements[0].scale = scale;
        state.captures[0].pixmap = Pixmap::new(
            (640.0 * scale).round() as u32,
            (480.0 * scale).round() as u32,
        )
        .unwrap()
        .into();
        state.selection.zone = Rect::from_xywh(values[0], values[1], values[2], values[3]);
        assert!(selection_size::apply::size(&mut state, (100, 80), &mut 0));
        let pixels = Pixmap::decode_png(&crate::app::render_final(&mut state).unwrap().0).unwrap();
        assert_eq!(
            (pixels.width(), pixels.height()),
            (
                sample["aligned"][2].as_u64().unwrap() as u32,
                sample["aligned"][3].as_u64().unwrap() as u32
            ),
            "{sample}"
        );
    }
}
