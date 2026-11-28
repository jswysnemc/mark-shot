//! 【截图选区测试】【双边吸附】通过真实输入验证选区边界和活动轴

use super::*;

/// 【截图选区测试】【画布边界】把手从边界内侧按下时仍按选区而非光标贴合画布右下角，无参数及返回值
#[test]
fn selection_snaps_both_canvas_edges_with_an_offset_grab() {
    let mut state = editor(true, false, 1.0);
    state.selection.zone = Rect::from_ltrb(160.0, 120.0, 450.0, 320.0);
    pointer(&mut state, 446.0, 316.0, &mut 0);
    button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 639.0, 479.0, &mut 0);
    button(&mut state, MouseButton::Left, false, &mut 0);
    assert_eq!(
        state.selection.zone,
        Rect::from_ltrb(160.0, 120.0, 640.0, 480.0)
    );
}

/// 【截图选区测试】【连续吸附】先贴底边再贴右边，分别移出容差可解除；边来自不同区域也能同时命中，无参数及返回值
#[test]
fn selection_corner_keeps_the_bottom_snap_while_acquiring_the_right() {
    for scale in [1.0, 1.5] {
        let mut state = editor(true, true, scale);
        state.selection.zone = Rect::from_ltrb(110.0, 110.0, 161.0, 150.0);
        pointer(&mut state, 161.0, 150.0, &mut 0);
        button(&mut state, MouseButton::Left, true, &mut 0);
        for (pointer_x, pointer_y, right, bottom) in [
            (191.0, 177.0, 191.0, 180.0),
            (237.0, 177.0, 240.0, 180.0),
            (397.0, 177.0, 400.0, 180.0),
            (397.0, 189.0, 400.0, 189.0),
            (415.0, 189.0, 415.0, 189.0),
        ] {
            pointer(&mut state, pointer_x, pointer_y, &mut 0);
            assert_eq!(
                state.selection.zone,
                Rect::from_ltrb(110.0, 110.0, right, bottom)
            );
        }
        button(&mut state, MouseButton::Left, false, &mut 0);
        assert_eq!(
            state.selection.zone,
            Rect::from_ltrb(110.0, 110.0, 415.0, 189.0)
        );
        assert!(state.annotations.is_empty());
    }
}

/// 【截图选区测试】【单边与禁用】拖动右边只改宽度；没有识别来源时仍按原始坐标自由调整，无参数及返回值
#[test]
fn selection_side_snap_changes_only_its_axis_and_respects_detection_settings() {
    for enabled in [false, true] {
        let mut state = editor(enabled, false, 1.0);
        state.selection.zone = Rect::from_ltrb(125.0, 106.0, 160.0, 177.0);
        pointer(&mut state, 160.0, 141.0, &mut 0);
        button(&mut state, MouseButton::Left, true, &mut 0);
        pointer(&mut state, 237.0, 177.0, &mut 0);
        button(&mut state, MouseButton::Left, false, &mut 0);
        let right = if enabled { 240.0 } else { 237.0 };
        assert_eq!(
            state.selection.zone,
            Rect::from_ltrb(125.0, 106.0, right, 177.0)
        );
    }
}

/// 【截图选区测试】【移动与锁定比例】移动整框保持尺寸，等比缩放保持对角锚点和宽高比，无参数及返回值
#[test]
fn selection_move_and_locked_corner_preserve_their_geometry() {
    let mut state = editor(true, false, 1.0);
    state.selection.zone = Rect::from_ltrb(120.0, 105.0, 190.0, 155.0);
    pointer(&mut state, 151.0, 129.0, &mut 0);
    button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 198.0, 152.0, &mut 0);
    button(&mut state, MouseButton::Left, false, &mut 0);
    assert_eq!(
        state.selection.zone,
        Rect::from_ltrb(170.0, 130.0, 240.0, 180.0)
    );
    state.selection.zone = Rect::from_ltrb(120.0, 100.0, 160.0, 140.0);
    state.selection.aspect_ratio = 1.0;
    pointer(&mut state, 160.0, 140.0, &mut 0);
    button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 197.0, 175.0, &mut 0);
    button(&mut state, MouseButton::Left, false, &mut 0);
    assert_eq!(
        state.selection.zone,
        Rect::from_ltrb(120.0, 100.0, 200.0, 180.0)
    );
}
