//! 【取色面板】【采样交互】悬停值与绘图颜色隔离，格式面板冻结采样，防止复制面板自身颜色

use super::{formats::Format, inspector_draw as layout};
use crate::{editor::EditorState, types::MouseButton, ui::panel::emit_panel_damage};
use tiny_skia::{Color, Rect};

pub struct Inspector {
    pub live: Option<Color>,
    pub panel: Option<(usize, Rect)>,
    pub recent: Vec<Color>,
    pub hover: Option<Hit>,
    pressed: Option<Hit>,
    pub fonts: Option<crate::ui::fonts::UiFonts>,
}

impl Default for Inspector {
    /// 【取色面板】【历史恢复】无参数，恢复共享颜色历史，不解析字库或创建面板
    fn default() -> Self {
        Self {
            live: None,
            panel: None,
            recent: crate::editor::saved::get()
                .recent_colors
                .iter()
                .take(8)
                .map(|color| color.color())
                .collect(),
            hover: None,
            pressed: None,
            fonts: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Copy(Format),
    Recent(usize),
    Close,
}

impl Inspector {
    /// 【取色面板】【控件命中】按 monitor、point 返回当前面板内的格式按钮、历史色或关闭按钮
    pub fn hit(&self, monitor: usize, point: (f64, f64)) -> Option<Hit> {
        let (owner, rect) = self.panel?;
        if owner != monitor {
            return None;
        }
        if contains(layout::close_rect(rect), point) {
            return Some(Hit::Close);
        }
        for (index, format) in Format::ALL.into_iter().enumerate() {
            if contains(layout::copy_rect(rect, index), point) {
                return Some(Hit::Copy(format));
            }
        }
        (0..self.recent.len())
            .find(|index| contains(layout::swatch_rect(rect, *index), point))
            .map(Hit::Recent)
    }

    /// 【取色面板】【历史更新】记住 color 并去重，最多八色；悬停不写历史
    pub fn remember(&mut self, color: Color) {
        self.recent.retain(|previous| *previous != color);
        self.recent.insert(0, color);
        self.recent.truncate(8);
    }
}

/// 【取色面板】【矩形判断】按 rect、point 返回是否命中，坐标均为显示器逻辑坐标
pub fn contains(rect: Rect, point: (f64, f64)) -> bool {
    point.0 >= f64::from(rect.left())
        && point.0 < f64::from(rect.right())
        && point.1 >= f64::from(rect.top())
        && point.1 < f64::from(rect.bottom())
}

/// 【取色面板】【显示面板】冻结 state 当前颜色并在指针附近展示完整格式，dirty 返回局部重绘范围
pub fn open(state: &mut EditorState, dirty: &mut u32) {
    let color = state
        .selected_annotation_index()
        .and_then(|index| state.annotations.get(index))
        .filter(|annotation| {
            matches!(
                annotation.shape,
                crate::types::AnnotationShape::ColorSample { .. }
            )
        })
        .map(|annotation| annotation.color)
        .or(state.color_inspector.live)
        .unwrap_or(state.tool_settings.color);
    state.color_inspector.live = Some(color);
    state.color_inspector.remember(color);
    state.color_popover.record_used_color(color);
    let monitor = state.input.pointer.monitor_idx;
    let size = state.placements[monitor].size;
    close(state, dirty);
    let rect = if let Some(anchor) = crate::ui::panel::UiPanel::rect(&state.settings_panel)
        .filter(|_| state.settings_panel.monitor_idx == monitor)
    {
        layout::dropdown_rect(size, anchor)
    } else {
        layout::panel_rect(size, state.input.pointer.local)
    };
    state.color_inspector.panel = Some((monitor, rect));
    state
        .color_inspector
        .fonts
        .get_or_insert_with(crate::ui::fonts::UiFonts::new);
    emit_panel_damage(rect, monitor, &mut state.damage_rects, dirty);
}

/// 【取色面板】【关闭面板】关闭 state 格式面板并恢复背景，dirty 返回旧范围；保留历史色与标注
pub fn close(state: &mut EditorState, dirty: &mut u32) {
    if let Some((monitor, rect)) = state.color_inspector.panel.take() {
        emit_panel_damage(rect, monitor, &mut state.damage_rects, dirty);
    }
    state.color_inspector.hover = None;
    state.color_inspector.pressed = None;
}

/// 【取色面板】【悬停同步】按 state 的真实指针更新控件高亮或原图采样，dirty 返回变化范围；返回是否由面板消费
pub fn pointer(state: &mut EditorState, dirty: &mut u32) -> bool {
    if let Some((monitor, rect)) = state.color_inspector.panel {
        let hit = state
            .color_inspector
            .hit(state.input.pointer.monitor_idx, state.input.pointer.local);
        if state.color_inspector.hover != hit {
            state.color_inspector.hover = hit;
            emit_panel_damage(rect, monitor, &mut state.damage_rects, dirty);
        }
        return true;
    }
    if state.picking()
        && !state.input.mouse_down
        && crate::app::input::canvas_ui_free(state)
        && let Some(color) = crate::tools::eyedropper::color_under_pointer(state)
        && state.color_inspector.live != Some(color)
    {
        state.color_inspector.live = Some(color);
        state.settings_panel.dirty = true;
        if let Some(rect) = crate::ui::panel::UiPanel::rect(&state.settings_panel) {
            emit_panel_damage(
                rect,
                state.settings_panel.monitor_idx,
                &mut state.damage_rects,
                dirty,
            );
        }
    }
    false
}

/// 【取色面板】【按钮路由】按 button、pressed 操作 state 面板，dirty 返回损坏；阻止点击穿透创建标注
pub fn button(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty: &mut u32,
) -> bool {
    let Some((monitor, rect)) = state.color_inspector.panel else {
        return false;
    };
    let hit = state
        .color_inspector
        .hit(state.input.pointer.monitor_idx, state.input.pointer.local);
    if pressed {
        if button == MouseButton::Right
            || !contains(rect, state.input.pointer.local)
            || monitor != state.input.pointer.monitor_idx
        {
            close(state, dirty);
        } else if button == MouseButton::Left {
            state.color_inspector.pressed = hit;
        }
    } else if button == MouseButton::Left
        && hit.is_some()
        && state.color_inspector.pressed.take() == hit
    {
        match hit.unwrap() {
            Hit::Close => close(state, dirty),
            Hit::Recent(index) => {
                state.color_inspector.live = Some(state.color_inspector.recent[index])
            }
            Hit::Copy(format) => {
                let text = format.text(state.color_inspector.live.unwrap());
                crate::tools::eyedropper::copy_text(state, &text);
            }
        }
        emit_panel_damage(rect, monitor, &mut state.damage_rects, dirty);
    }
    true
}
