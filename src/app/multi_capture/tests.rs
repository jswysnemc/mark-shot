//! 【多选区截图】【真实输入测试】核对区域运算、屏幕预览与最终 PNG 的选中像素

use crate::{
    app::{capture_start, input, render_final},
    editor::EditorState,
    keys::{Chord, Key, Mods},
    types::MouseButton,
    ui::capture_start::{Control, layout},
};
use tiny_skia::{Color, Pixmap};

/// 【多选区截图】【测试入口】构建单屏冻结图，通过起始工具栏的实际按下与释放进入多选区模式
fn editor() -> EditorState {
    let mut state = input::tests::editor();
    let mut pixels = Pixmap::new(640, 480).unwrap();
    pixels.fill(Color::from_rgba8(30, 180, 90, 255));
    state.captures[0].pixmap = pixels.into();
    capture_start::initialize(&mut state, &Default::default());
    let control = layout::controls(state.placements[0].size, false)
        .into_iter()
        .find(|(kind, _)| *kind == Control::MultiRegion)
        .unwrap()
        .1;
    input::handle_pointer_move(
        &mut state,
        0,
        (control.left() + 8.0).into(),
        (control.top() + 8.0).into(),
        &mut 0,
    );
    for pressed in [true, false] {
        input::handle_pointer_button(&mut state, MouseButton::Left, pressed, &mut 0);
    }
    assert!(state.multi_capture.is_some());
    state
}

/// 【多选区截图】【拖选输入】按 button 从 start 拖到 end 并松开，经过原生事件使用的分派入口
fn drag(state: &mut EditorState, button: MouseButton, start: (f64, f64), end: (f64, f64)) {
    input::handle_pointer_move(state, 0, start.0, start.1, &mut 0);
    input::handle_pointer_button(state, button, true, &mut 0);
    input::handle_pointer_move(state, 0, end.0, end.1, &mut 0);
    input::handle_pointer_button(state, button, false, &mut 0);
}

/// 【多选区截图】【键盘输入】按 key 与 ctrl 状态发送一次键盘事件，返回值为空
fn key(state: &mut EditorState, key: Key, ctrl: bool) {
    input::handle_key(
        state,
        Some(Chord {
            key,
            mods: Mods {
                ctrl,
                ..Default::default()
            },
        }),
        None,
        &mut 0,
    );
}

/// 【多选区截图】【透明裁切】交叠区域只亮一次，空隙和内部孔洞透明；撤销恢复真实像素而非扩大外接框
#[test]
fn overlapping_and_separate_regions_keep_holes_in_preview_and_png() {
    let mut state = editor();
    for (start, end) in [
        ((100.0, 100.0), (150.0, 160.0)),
        ((140.0, 130.0), (180.0, 160.0)),
        ((230.0, 120.0), (210.0, 100.0)),
    ] {
        drag(&mut state, MouseButton::Left, start, end);
    }
    drag(
        &mut state,
        MouseButton::Right,
        (110.0, 110.0),
        (120.0, 120.0),
    );
    super::preview::draw(&mut state, 0, None);
    assert_eq!(state.canvas[0].pixel(145, 140).unwrap().green(), 180);
    assert!(state.canvas[0].pixel(115, 115).unwrap().green() < 180);
    let output =
        |state: &mut EditorState| Pixmap::decode_png(&render_final(state).unwrap().0).unwrap();
    let masked = output(&mut state);
    assert_eq!((masked.width(), masked.height()), (130, 60));
    for (column, row, alpha) in [(15, 15, 0), (45, 40, 255), (95, 15, 0), (115, 10, 255)] {
        assert_eq!(masked.pixel(column, row).unwrap().alpha(), alpha);
    }
    assert!(crate::app::history_source::png(&state).is_none());
    key(&mut state, Key::Char('z'), true);
    assert_eq!(output(&mut state).pixel(15, 15).unwrap().alpha(), 255);
    key(&mut state, Key::Char('c'), true);
    assert!(state.finish.is_some_and(|outputs| outputs.copy));
}

/// 【多选区截图】【确认后编辑】通过真实拖选与确认键进入下一阶段，确认本身不能触发复制、保存或关闭
#[test]
fn enter_after_a_valid_selection_does_not_deliver_the_capture() {
    let mut state = editor();
    state.to = Default::default();
    for (button, start, end) in [
        (MouseButton::Left, (100.0, 100.0), (145.0, 140.0)),
        (MouseButton::Left, (180.0, 100.0), (205.0, 128.0)),
        (MouseButton::Right, (110.0, 110.0), (119.0, 119.0)),
    ] {
        drag(&mut state, button, start, end);
    }
    key(&mut state, Key::Enter, false);
    assert!(
        state.finish.is_none() && !state.cancel,
        "确认选区应进入标注编辑，不应直接交付截图"
    );
    let original = super::take_image(&mut state).expect("应向标注窗口交接选区原图");
    assert_eq!((original.width(), original.height()), (105, 40));
    for (column, row, expected) in [
        (4, 4, [30, 180, 90, 255]),
        (90, 10, [30, 180, 90, 255]),
        (14, 14, [0; 4]),
        (60, 20, [0; 4]),
    ] {
        let pixel = original.pixel(column, row).unwrap();
        assert_eq!(
            [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()],
            expected
        );
    }
    assert!(super::take_image(&mut state).is_none());
}

/// 【多选区截图】【按钮交接】仅在可用按钮内按下并松开才进入编辑；拖选经过按钮仍按普通选区提交
#[test]
fn edit_button_requires_a_complete_click_after_selection() {
    let mut state = editor();
    let button = super::bar::Bar::new(state.placements[0].rect().unwrap())
        .unwrap()
        .edit;
    let inside = (f64::from(button.x() + 6.0), f64::from(button.y() + 6.0));
    let outside = (inside.0 - 40.0, inside.1 - 30.0);
    drag(&mut state, MouseButton::Left, inside, inside);
    assert!(super::take_image(&mut state).is_none());
    drag(&mut state, MouseButton::Left, outside, inside);
    assert!(super::take_image(&mut state).is_none());
    assert!(super::edit_hit(&state));
    input::handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    assert!(super::take_image(&mut state).is_none());
    input::handle_pointer_move(&mut state, 0, outside.0, outside.1, &mut 0);
    input::handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    assert!(super::take_image(&mut state).is_none());
    drag(&mut state, MouseButton::Left, inside, inside);
    let original = super::take_image(&mut state).expect("点击进入编辑后应交接选区原图");
    assert_eq!((original.width(), original.height()), (40, 30));
    assert!(state.finish.is_none() && !state.cancel);
}

/// 【多选区截图】【手势取消】清空或尚未确认的选区不能退回全屏导出；Esc 依次取消草稿和多选区模式
#[test]
fn empty_mask_and_cancelled_drag_never_fall_back_to_fullscreen() {
    let mut state = editor();
    drag(&mut state, MouseButton::Left, (50.0, 50.0), (100.0, 100.0));
    drag(&mut state, MouseButton::Right, (30.0, 30.0), (110.0, 110.0));
    key(&mut state, Key::Enter, false);
    assert!(state.finish.is_none() && render_final(&mut state).is_none());
    assert!(super::take_image(&mut state).is_none());
    input::handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    input::handle_pointer_move(&mut state, 0, 200.0, 210.0, &mut 0);
    key(&mut state, Key::Enter, false);
    assert!(state.finish.is_none());
    assert!(super::take_image(&mut state).is_none());
    key(&mut state, Key::Escape, false);
    assert!(
        state
            .multi_capture
            .as_ref()
            .unwrap()
            .regions
            .visible()
            .is_empty()
    );
    key(&mut state, Key::Escape, false);
    assert!(state.multi_capture.is_none() && !state.cancel && state.selection.zone.is_none());
}

/// 【多选区截图】【跨屏交付】左侧负坐标显示器和右侧分数倍率显示器共用物理网格，空隙的 RGB 与透明度均清零
#[test]
fn mixed_scale_outputs_preserve_origin_and_clear_unselected_channels() {
    use crate::types::{Capture, Placement};
    use tiny_skia::Rect;
    let placements = [
        Placement {
            position: (-40, 0),
            size: (40, 32),
            native_size: None,
            scale: 1.0,
        },
        Placement {
            position: (0, 0),
            size: (40, 32),
            native_size: None,
            scale: 1.5,
        },
    ];
    let captures = placements
        .iter()
        .zip([
            Color::from_rgba8(15, 190, 40, 255),
            Color::from_rgba8(75, 55, 203, 255),
        ])
        .map(|(placement, color)| {
            let (width, height) = placement.px_size();
            let mut pixels = Pixmap::new(width, height).unwrap();
            pixels.fill(color);
            Capture {
                pixmap: pixels.into(),
                scale: placement.scale,
            }
        })
        .collect::<Vec<_>>();
    let regions = [
        Rect::from_ltrb(-12.0, 4.0, -4.0, 12.0).unwrap(),
        Rect::from_ltrb(4.0, 6.0, 12.0, 14.0).unwrap(),
    ];
    let (png, origin, scale) =
        crate::features::multi_capture::export::render(&captures, &placements, &regions).unwrap();
    assert_eq!((origin, scale), ((-12.0, 4.0), 1.5));
    let result = image::load_from_memory(&png).unwrap().into_rgba8();
    assert_eq!(result.dimensions(), (36, 15));
    assert_eq!(result.get_pixel(4, 4).0, [15, 190, 40, 255]);
    assert_eq!(result.get_pixel(17, 5).0, [0; 4]);
    assert_eq!(result.get_pixel(28, 6).0, [75, 55, 203, 255]);
}

/// 【多选区截图】【单区恢复】隐藏起始栏时仍能通过快捷键进入；退出后保留原单选区，普通截图仍输出完整矩形
#[test]
fn shortcut_restores_the_existing_single_region_on_exit() {
    use tiny_skia::Rect;
    let mut state = input::tests::editor();
    capture_start::initialize(&mut state, &Default::default());
    state.capture_start.visible = false;
    let original = Rect::from_xywh(220.0, 160.0, 96.0, 64.0);
    state.selection.zone = original;
    let chord = Chord {
        key: Key::Char('m'),
        mods: Mods {
            ctrl: true,
            shift: true,
            ..Default::default()
        },
    };
    input::handle_key(&mut state, Some(chord), None, &mut 0);
    assert!(state.multi_capture.is_some());
    input::handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    assert!(render_final(&mut state).is_none());
    super::cancel_drag(&mut state, &mut 0);
    drag(
        &mut state,
        MouseButton::Right,
        (250.0, 180.0),
        (260.0, 190.0),
    );
    super::escape(&mut state, &mut 0);
    assert_eq!(state.selection.zone, original);
    let pixels = Pixmap::decode_png(&render_final(&mut state).unwrap().0).unwrap();
    assert_eq!((pixels.width(), pixels.height()), (96, 64));
    assert_eq!(pixels.pixel(35, 25).unwrap().alpha(), 255);
}
