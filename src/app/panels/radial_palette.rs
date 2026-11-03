//! 【快捷色板】【编辑交接】原版右键双击色板锚定指针，左键原位释放才确认颜色

use crate::{
    editor::{EditorState, dirty::mark_all_dirty},
    tools::Tool,
    types::StrokeGroup,
    ui::{
        color_popover::ColorTarget,
        panel::UiPanel,
        radial_palette::{COLORS, RadialPalette, SIZE},
    },
};

/// 【快捷色板】【显示切换】在 state 当前指针处切换快捷色板；dirty 接收旧、新区域，不跟随工具栏
pub fn toggle(state: &mut EditorState, dirty: &mut u32) {
    super::text::font_popup::close(state, dirty);
    if state.color_popover.open && state.color_popover.radial.is_some() {
        super::color::close_color_popover(state, dirty);
        return;
    }
    super::color::close_color_popover(state, dirty);
    let monitor = state.input.pointer.monitor_idx;
    let anchor = (
        state.input.pointer.local.0 as f32,
        state.input.pointer.local.1 as f32,
    );
    state.color_popover.radial = Some(RadialPalette {
        anchor,
        monitor,
        ..Default::default()
    });
    state.color_popover.target = ColorTarget::Main;
    state.color_popover.select_color(state.tool_settings.color);
    state.color_popover.size = (SIZE, SIZE);
    state.color_popover.open = true;
    super::color::update_color_popover(state, dirty);
}

/// 【快捷色板】【预览尺寸】返回 state 当前工具的 Qt 预览尺寸；选择和移动使用八像素方块
fn preview_size(state: &EditorState) -> f32 {
    let tool = state.selected_tool;
    let width = state.tool_settings.stroke(StrokeGroup::of_tool(tool));
    match tool {
        Tool::Selection | Tool::Pick => 8.0,
        Tool::Text => (19.0 + state.tool_settings.font_size).max(10.0),
        Tool::Number => ((13.0 + width * 1.35) * 2.0).max(26.0),
        Tool::Highlighter => width.max(6.0),
        Tool::Laser => width.max(8.0),
        Tool::Redact => state.tool_settings.strength.max(2.0),
        _ => width.max(1.5),
    }
}

/// 【快捷色板】【锚点刷新】根据 state 更新固定位置、悬停和笔宽预览；返回是否变化，不调用捕获或创建线程
pub fn update(state: &mut EditorState) -> bool {
    let preview = preview_size(state);
    let Some(radial) = &mut state.color_popover.radial else {
        return false;
    };
    let bounds = state.placements[radial.monitor].size;
    let position = radial.position(bounds);
    let hovered = (state.input.pointer.monitor_idx == radial.monitor)
        .then(|| {
            RadialPalette::hit((
                state.input.pointer.local.0 - position.0 as f64,
                state.input.pointer.local.1 - position.1 as f64,
            ))
        })
        .flatten();
    let changed = radial.preview_size != preview || radial.hovered != hovered;
    radial.preview_size = preview;
    radial.hovered = hovered;
    state.color_popover.position = position;
    state.color_popover.render_pos = position;
    state.color_popover.monitor_idx = radial.monitor;
    state.color_popover.size = (SIZE, SIZE);
    changed
}

/// 【快捷色板】【左键按下】在 state 中只记录按钮，不提前改变颜色；返回是否为快捷色板事件
pub fn press(state: &mut EditorState) -> bool {
    let Some(rect) = state.color_popover.rect() else {
        return false;
    };
    let Some(radial) = &mut state.color_popover.radial else {
        return false;
    };
    radial.armed = RadialPalette::hit((
        state.input.pointer.local.0 - rect.left() as f64,
        state.input.pointer.local.1 - rect.top() as f64,
    ));
    true
}

/// 【快捷色板】【左键释放】按 state 当前实际指针确认同一按钮；dirty 返回颜色预览变化，不修改原图
pub fn release(state: &mut EditorState, dirty: &mut u32) -> bool {
    let Some(rect) = state.color_popover.rect() else {
        return false;
    };
    let Some(radial) = &mut state.color_popover.radial else {
        return false;
    };
    let armed = radial.armed.take();
    let hit = (state.input.pointer.monitor_idx == radial.monitor)
        .then(|| {
            RadialPalette::hit((
                state.input.pointer.local.0 - rect.left() as f64,
                state.input.pointer.local.1 - rect.top() as f64,
            ))
        })
        .flatten();
    if let Some(index) = armed.filter(|index| Some(*index) == hit) {
        let color = COLORS[index].color();
        crate::editor::color_selection::apply_quick_color(state, color, dirty);
        super::color::close_color_popover(state, dirty);
        mark_all_dirty(dirty, state.placements.len());
    }
    true
}
