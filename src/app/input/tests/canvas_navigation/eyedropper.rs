use super::*;

/// 【取色测试】【圆镜色块】按编辑器、显示器内指针及预期色值核验读数，只读取信息条内部像素，无返回值
fn assert_swatch(state: &mut EditorState, cursor: (f64, f64), expected: Color) {
    draw(state, None);
    let rect =
        crate::ui::magnifier::magnifier_rect((cursor.0 as f32, cursor.1 as f32), 640.0, 480.0);
    let scale = state.placements[0].scale;
    let color = expected.premultiply().to_color_u8();
    let row = ((rect.bottom() - crate::config::get().magnifier.label_height.min(20.0) * 1.5)
        * scale) as u32;
    assert!(
        (((rect.left() + 8.0) * scale) as u32..((rect.right() - 8.0) * scale) as u32)
            .filter(|&column| state.canvas[0].pixel(column, row) == Some(color))
            .count()
            >= 6
    );
}

/// 【取色测试】【缩放采样】缩放并平移后，圆镜读数与实际采样都应使用原图，而不是显示器上的同名坐标或预览截图
#[test]
fn loupe_and_annotation_sample_the_same_zoomed_original() {
    let mut state = image_editor(1.5);
    let mut dirty = 0;
    pointer(&mut state, 200.0, 155.0, &mut dirty);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(720), &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Middle, true, &mut dirty);
    pointer(&mut state, 218.0, 168.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Middle, false, &mut dirty);
    select_tool(&mut state, Tool::Eyedropper, &mut dirty);
    let cursor = (302.0, 210.0);
    pointer(&mut state, cursor.0, cursor.1, &mut dirty);
    let point = state.canvas_point();
    let file = state.file_canvas.as_ref().unwrap();
    let native = (
        (point.0 - f64::from(file.viewport.left())) * 320.0 / f64::from(file.viewport.width()),
        (point.1 - f64::from(file.viewport.top())) * 240.0 / f64::from(file.viewport.height()),
    );
    let expected = crate::ui::magnifier::sample_pixel(&file.original, native).unwrap();
    assert_eq!(state.color_inspector.live, Some(expected));
    // 1. 【取色测试】【预览核验】只统计圆镜读数范围的原图色块，排除正文及十字线
    assert_swatch(&mut state, cursor, expected);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    let AnnotationShape::ColorSample { sample } = state.pending.as_ref().unwrap().shape else {
        panic!("expected sample")
    };
    assert_eq!(sample.color, expected);
    assert_eq!(sample.source, (point.0 as f32, point.1 as f32));
}

/// 【取色测试】【桌面整页缩放】负原点显示器放大后，圆镜与调色板单次取色都命中原截图的物理像素
#[test]
fn zoomed_desktop_loupe_and_palette_pick_share_the_native_pixel() {
    let mut state = image_editor(1.5);
    state.file_canvas = None;
    state.placements[0].position = (-640, -20);
    state.selection.zone = crate::utils::get_full_workspace_rect(&state.placements);
    let mut source = Pixmap::new(960, 720).unwrap();
    for (offset, pixel) in source
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        *pixel = [
            (offset % 960 % 251) as u8,
            (offset / 960 % 241) as u8,
            170,
            255,
        ];
    }
    state.captures[0].pixmap = source.into();
    state.captures[0].scale = 1.5;
    select_tool(&mut state, Tool::Pick, &mut 0);
    pointer(&mut state, 200.0, 155.0, &mut 0);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(720), &mut 0);
    assert!(state.canvas_transform().scale > 1.0);
    crate::tools::eyedropper::toggle_pick_once(&mut state, &mut 0);
    let cursor = (302.0, 210.0);
    pointer(&mut state, cursor.0, cursor.1, &mut 0);
    let (horizontal, vertical) = state.canvas_point();
    let expected = Color::from_rgba8(
        (((horizontal + 640.0) * 1.5).floor() as u32 % 251) as u8,
        (((vertical + 20.0) * 1.5).floor() as u32 % 241) as u8,
        170,
        255,
    );
    assert_swatch(&mut state, cursor, expected);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    assert_eq!(state.tool_settings.color, expected);
    assert!(state.annotations.is_empty());
}
