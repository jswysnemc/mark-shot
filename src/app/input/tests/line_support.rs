//! 【线条测试】【输入辅助】构造明确偏好，复用真实输入分派和样式网格位置

use super::{editor, key, pointer};
use crate::app::input::{handle_pointer_button, select_tool};
use crate::app::panels;
use crate::editor::EditorState;
use crate::features::{
    arrow::ArrowStyle,
    line::{LineSkeleton, LineStyle},
};
use crate::tools::Tool;
use crate::types::{MouseButton, StrokeGroup};
use crate::ui::{line_properties::LineChoice, panel::AnimatedPanel};

/// 【线条测试】【编辑器构造】返回使用 tool 的编辑器，显式设置独立笔宽和默认样式；dirty 接收初始化重绘
pub(super) fn line_editor(tool: Tool, dirty: &mut u32) -> EditorState {
    let mut state = editor();
    state.tool_settings.arrow_style = ArrowStyle::Fletched;
    state.tool_settings.arrow_line_style = LineStyle::Solid;
    state.tool_settings.line_style = LineStyle::Solid;
    state.tool_settings.set_stroke(StrokeGroup::Arrow, 3.0);
    state.tool_settings.set_stroke(StrokeGroup::Line, 3.0);
    state.tool_settings.set_stroke(StrokeGroup::Numbered, 11.0);
    state.tool_settings.set_stroke(StrokeGroup::Freehand, 7.0);
    state.tool_settings.color = tiny_skia::Color::from_rgba8(220, 40, 50, 255);
    key(
        &mut state,
        if tool == Tool::Arrow { "A" } else { "D" },
        dirty,
    );
    state.toolbar.opacity = 1.0;
    state.toolbar.render_pos = state.toolbar.position;
    panels::settings::update_settings_panel(&mut state, dirty);
    state
}

/// 【线条测试】【绘图手势】在 state 中从 start 拖到 end 并提交；dirty 接收新旧范围，无返回值
pub(super) fn draw(state: &mut EditorState, start: (f64, f64), end: (f64, f64), dirty: &mut u32) {
    pointer(state, start.0, start.1, dirty);
    handle_pointer_button(state, MouseButton::Left, true, dirty);
    pointer(state, end.0, end.1, dirty);
    handle_pointer_button(state, MouseButton::Left, false, dirty);
}

/// 【线条测试】【点击手势】在 state 的 point 发送一次左键按下和释放；dirty 接收重绘，无返回值
pub(super) fn click(state: &mut EditorState, point: (f64, f64), dirty: &mut u32) {
    pointer(state, point.0, point.1, dirty);
    handle_pointer_button(state, MouseButton::Left, true, dirty);
    handle_pointer_button(state, MouseButton::Left, false, dirty);
}

/// 【线条测试】【骨架读取】返回 state 中 index 标注的骨架；不接受其他工具形状
pub(super) fn skeleton(state: &EditorState, index: usize) -> &LineSkeleton {
    state.annotations[index]
        .shape
        .line()
        .expect("line skeleton expected")
}

/// 【线条测试】【选中标注】在 state 中进入选择工具并选中 index，刷新对应属性；dirty 接收重绘，无返回值
pub(super) fn pick(state: &mut EditorState, index: usize, dirty: &mut u32) {
    select_tool(state, Tool::Pick, dirty);
    state.selected_annotation = Some(index);
    crate::app::input::refresh_panels(state, dirty);
}

/// 【线条测试】【弹层打开】在 state 中打开 kind 并完成透明度动画；dirty 接收重绘，无返回值
pub(super) fn open(state: &mut EditorState, kind: LineChoice, dirty: &mut u32) {
    panels::line::toggle(state, kind, dirty);
    match kind {
        LineChoice::Heads => {
            state.arrow_popover.animate_step(1.0);
        }
        LineChoice::Pattern => {
            state.line_popover.animate_step(1.0);
        }
    }
    panels::line::update(state, dirty);
}

/// 【线条测试】【样式确认】在 state 的 kind 弹层中按键选择 index 并确认；dirty 接收重绘，无返回值
pub(super) fn choose(state: &mut EditorState, kind: LineChoice, index: usize, dirty: &mut u32) {
    open(state, kind, dirty);
    key(state, "Home", dirty);
    for _ in 0..index {
        key(state, "Right", dirty);
    }
    key(state, "Enter", dirty);
}

/// 【线条测试】【条目中心】返回 state 的 kind 弹层中 index 可见条目的本地中心
pub(super) fn item_center(state: &EditorState, kind: LineChoice, index: usize) -> (f64, f64) {
    let panel = panels::line::popup(state, kind);
    let rect = panel.layout.item_rect(index, panel.first_row).unwrap();
    (
        (panel.render_pos.0 + rect.left() + rect.width() * 0.5) as f64,
        (panel.render_pos.1 + rect.top() + rect.height() * 0.5) as f64,
    )
}
