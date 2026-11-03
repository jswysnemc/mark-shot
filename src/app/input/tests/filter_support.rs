//! 【滤镜测试】【输入辅助】构造明确默认值，复用真实输入分派与模式弹层

use super::{editor, key, pointer};
use crate::app::{
    input::{handle_pointer_button, select_tool},
    panels,
};
use crate::editor::EditorState;
use crate::features::filter::{FilterMode, RegionFilter};
use crate::tools::Tool;
use crate::types::{AnnotationShape, MouseButton};
use crate::ui::panel::AnimatedPanel;

/// 【滤镜测试】【编辑器构造】返回已选择滤镜的编辑器，默认像素化和十四强度；dirty 返回初始化损坏范围
pub(super) fn filter_editor(dirty: &mut u32) -> EditorState {
    let mut state = editor();
    state.tool_settings.filter_mode = FilterMode::Pixelate;
    state.tool_settings.strength = 14.0;
    state.tool_settings.color = tiny_skia::Color::from_rgba8(220, 40, 50, 255);
    key(&mut state, "M", dirty);
    state.toolbar.opacity = 1.0;
    state.toolbar.render_pos = state.toolbar.position;
    panels::settings::update_settings_panel(&mut state, dirty);
    state
}

/// 【滤镜测试】【绘图手势】在 state 中从 start 拖到 end 并提交；dirty 返回新旧重绘范围，无返回值
pub(super) fn draw(state: &mut EditorState, start: (f64, f64), end: (f64, f64), dirty: &mut u32) {
    pointer(state, start.0, start.1, dirty);
    handle_pointer_button(state, MouseButton::Left, true, dirty);
    pointer(state, end.0, end.1, dirty);
    handle_pointer_button(state, MouseButton::Left, false, dirty);
}

/// 【滤镜测试】【区域读取】返回 state 中 index 对应的滤镜，不接受其他标注类型
pub(super) fn filter(state: &EditorState, index: usize) -> RegionFilter {
    let AnnotationShape::Redact { filter } = state.annotations[index].shape else {
        panic!("filter expected")
    };
    filter
}

/// 【滤镜测试】【选中区域】在 state 中进入选择工具并选中 index；dirty 返回属性栏和控制框范围，无返回值
pub(super) fn pick(state: &mut EditorState, index: usize, dirty: &mut u32) {
    select_tool(state, Tool::Pick, dirty);
    state.selected_annotation = Some(index);
    crate::app::input::refresh_panels(state, dirty);
}

/// 【滤镜测试】【面板打开】打开 state 模式弹层并完成统一透明度动画；dirty 返回范围，无返回值
pub(super) fn open(state: &mut EditorState, dirty: &mut u32) {
    panels::filter::toggle(state, dirty);
    state.filter_popover.animate_step(1.0);
    panels::filter::update(state, dirty);
}

/// 【滤镜测试】【键盘确认】在 state 模式弹层中选择 index 并确认；dirty 返回范围，无返回值
pub(super) fn choose(state: &mut EditorState, index: usize, dirty: &mut u32) {
    open(state, dirty);
    key(state, "Home", dirty);
    for _ in 0..index {
        key(state, "Right", dirty);
    }
    key(state, "Enter", dirty);
}
