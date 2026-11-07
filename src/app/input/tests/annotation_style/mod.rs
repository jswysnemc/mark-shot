//! 【标注样式测试】【输入环境】使用真实工具分派验证原图笔宽，不连接显示服务器

mod creation;
mod export;
mod hit;
mod hit_reference;
mod navigation;
mod properties;
mod ruler_chrome;
mod ruler_labels;
mod ruler_performance;
mod ruler_reference;

use super::{editor, key, pointer, widget_center};
use crate::{
    app::input::{handle_pointer_button, select_tool},
    editor::EditorState,
    tools::Tool,
    types::{MouseButton, StrokeGroup},
};
use tiny_skia::{Color, Pixmap, Rect};

const TOOLS: [Tool; 11] = [
    Tool::Rectangle,
    Tool::Circle,
    Tool::Arrow,
    Tool::Line,
    Tool::Pen,
    Tool::Highlighter,
    Tool::Number,
    Tool::Marker,
    Tool::Magnifier,
    Tool::Ruler,
    Tool::NumeratedArrow,
];

/// 【标注样式测试】【文件环境】按size返回适配六百四十乘四百八十窗口的文件编辑器，原图使用白色像素
fn image_editor(size: (u32, u32)) -> EditorState {
    let mut state = editor();
    let mut original = Pixmap::new(size.0, size.1).unwrap();
    original.fill(Color::WHITE);
    let file = crate::editor::file_canvas::FileCanvas::new(
        original,
        Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap(),
    )
    .unwrap();
    state.selection.zone = Some(file.viewport);
    state.file_canvas = Some(file);
    state
}

/// 【标注样式测试】【截图环境】按scale返回原图像素与窗口逻辑尺寸分离的截图编辑器
fn capture_editor(scale: f32) -> EditorState {
    let mut state = editor();
    state.placements[0].scale = scale;
    state.captures[0].scale = scale;
    let mut image = Pixmap::new((640.0 * scale) as u32, (480.0 * scale) as u32).unwrap();
    image.fill(Color::WHITE);
    state.captures[0].pixmap = image.clone().into();
    state.canvas[0] = image.clone();
    state.dimmed[0] = image;
    state.annotations_layer[0] =
        Pixmap::new((640.0 * scale) as u32, (480.0 * scale) as u32).unwrap();
    state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    state
}

/// 【标注样式测试】【绘制手势】按tool和原图width在state中创建标注，dirty返回输入损坏范围，无返回值
fn draw(state: &mut EditorState, tool: Tool, width: f32, dirty: &mut u32) {
    state.tool_settings.ruler_mode = crate::features::ruler::RulerMode::Length;
    select_tool(state, tool, dirty);
    state
        .tool_settings
        .set_stroke(StrokeGroup::of_tool(tool), width);
    state.tool_settings.color = Color::BLACK;
    pointer(state, 220.0, 160.0, dirty);
    handle_pointer_button(state, MouseButton::Left, true, dirty);
    for point in [(250.0, 180.0), (320.0, 220.0), (390.0, 300.0)] {
        pointer(state, point.0, point.1, dirty);
    }
    handle_pointer_button(state, MouseButton::Left, false, dirty);
    assert!(state.pending.is_none(), "tool={tool:?}");
    assert!(!state.annotations.is_empty(), "tool={tool:?}");
}

/// 【标注样式测试】【精度比较】按actual、expected与context检查浮点换算结果，无返回值
fn close(actual: f32, expected: f32, context: impl std::fmt::Debug) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "{context:?}: {actual} != {expected}"
    );
}
