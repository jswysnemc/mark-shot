//! 【标注样式测试】【原图导出】不同初始适配比例导出同样原图笔宽，导出与导航状态无关

use super::*;

/// 【标注样式测试】【导出厚度】小图放大和大图缩小创建六像素水平线，原图导出中部正文厚度保持六像素
#[test]
fn exported_line_width_is_six_image_pixels_for_both_initial_fit_directions() {
    for size in [(320, 240), (1280, 960)] {
        let mut state = image_editor(size);
        let mut dirty = 0;
        select_tool(&mut state, Tool::Line, &mut dirty);
        state.tool_settings.set_stroke(StrokeGroup::Line, 6.0);
        state.tool_settings.color = Color::BLACK;
        pointer(&mut state, 220.0, 240.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        pointer(&mut state, 390.0, 240.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        let png = crate::app::image_export::render_precise(&mut state)
            .unwrap()
            .0;
        let pixels = Pixmap::decode_png(&png).unwrap();
        assert_eq!((pixels.width(), pixels.height()), size);
        let column = size.0 / 2;
        let dark = (0..size.1)
            .filter(|y| pixels.pixel(column, *y).unwrap().red() < 100)
            .count();
        assert_eq!(dark, 6, "size={size:?}");
    }
}
