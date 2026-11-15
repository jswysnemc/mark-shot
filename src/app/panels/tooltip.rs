//! 【工具提示】【悬停交互】管理按钮命中、有效快捷键、显示期限及局部重绘

use crate::{
    editor::EditorState,
    ui::{
        panel::{UiPanel, emit_panel_damage},
        toolbar::{Orientation, ToolbarItem, catalog},
        tooltip::{Target, View},
    },
};
use std::time::Instant;

/// 【工具提示】【目标命中】按 state 返回当前实际按钮，忙碌、隐藏或弹层覆盖时返回空值；禁用按钮仍可查看名称
fn target(state: &EditorState) -> Option<Target> {
    // 1. 【工具提示】【交互保护】正在操作画布或独立弹层时不追踪工具栏提示
    if state.ui_hidden
        || state.tool_active
        || state.input.mouse_down
        || state.input.toolbar_press.is_some()
        || state.ann_drag.is_some()
        || state.toolbar_drag.is_some()
        || state.navigation.pan.is_some()
        || state.navigation.slider.is_some()
        || state.collection.drag.is_some()
        || state.text_context_menu.is_some()
        || state.external_panel.kind.is_some()
        || state.upload.busy()
        || state.selection_translation.busy()
        || state.selection_size.interacting()
    {
        return None;
    }
    // 2. 【工具提示】【命中归属】沿用统一弹层优先级，仅工具栏实际按钮可以生成提示
    let secondary = match crate::app::input::ui_hit::hit_test_ui(state, state.input.pointer.local) {
        crate::app::input::ui_hit::UiHit::ToolbarItem(_) => false,
        crate::app::input::ui_hit::UiHit::ActionToolbarItem(_) => true,
        _ => return None,
    };
    let toolbar = if secondary {
        &state.action_toolbar
    } else {
        &state.toolbar
    };
    if toolbar.interferes {
        return None;
    }
    let index = toolbar.hit_test(state.input.pointer.local).1?;
    let ToolbarItem::Button(button) = *toolbar.items.get(index)? else {
        return None;
    };
    // 3. 【工具提示】【屏幕锚点】将面板内部按钮矩形转换为显示器本地坐标
    let cell = toolbar.cell(index)?;
    Some(Target {
        button,
        secondary,
        monitor_idx: toolbar.monitor_idx,
        cell: tiny_skia::Rect::from_xywh(
            cell.left() + toolbar.render_pos.0,
            cell.top() + toolbar.render_pos.1,
            cell.width(),
            cell.height(),
        )?,
        toolbar: toolbar.rect()?,
        vertical: toolbar.orientation == Orientation::Vertical,
    })
}

/// 【工具提示】【按钮文案】按 button、language 与 keymap 返回名称和实际快捷键；未绑定时只返回名称
pub(crate) fn label(
    button: crate::ui::toolbar::ToolbarButton,
    language: crate::ui::language::Language,
    keymap: &crate::keys::Keymap,
) -> Option<String> {
    let entry = catalog::entry(ToolbarItem::Button(button))?;
    let name = entry.label(language == crate::ui::language::Language::Chinese);
    Some(
        match button.action().and_then(|action| keymap.label(action)) {
            Some(key) => format!("{name} ({key})"),
            None => name.to_owned(),
        },
    )
}

/// 【工具提示】【关闭恢复】取消 state 的显示和期限，dirty 返回需要恢复旧提示区域的显示器，无返回值
pub(crate) fn close(state: &mut EditorState, dirty: &mut u32) {
    if let Some((monitor, rect)) = state.toolbar_tooltip.close() {
        emit_panel_damage(rect, monitor, &mut state.damage_rects, dirty);
    }
}

/// 【工具提示】【鼠标更新】按 state 的当前屏幕指针及 now 更新悬停目标，dirty 返回旧提示损坏范围，无返回值
pub(crate) fn pointer_move(state: &mut EditorState, now: Instant, dirty: &mut u32) {
    let target = target(state);
    if let Some((monitor, rect)) = state.toolbar_tooltip.hover(target, now) {
        emit_panel_damage(rect, monitor, &mut state.damage_rects, dirty);
    }
}

/// 【工具提示】【期限显示】按 now 校验 state 的原目标并创建到期提示，dirty 返回新旧显示范围，无返回值
pub(crate) fn tick(state: &mut EditorState, now: Instant, dirty: &mut u32) {
    // 1. 【工具提示】【目标校验】点击关闭后不自行重新开启；工具栏移动或弹层变化会撤销旧目标
    if !state.toolbar_tooltip.tracking() {
        return;
    }
    pointer_move(state, now, dirty);
    if !state.toolbar_tooltip.due(now) {
        return;
    }
    let Some(target) = state.toolbar_tooltip.target() else {
        return;
    };
    let Some(text) = label(
        target.button,
        crate::ui::language::Language::current(),
        &crate::keys::map(),
    ) else {
        close(state, dirty);
        return;
    };
    // 2. 【工具提示】【文字准备】仅在到期显示时准备中文字形与尺寸，不在鼠标每次移动时加载字体
    crate::fonts::ui_fallback::ensure_text(&mut state.text.font_system, &text);
    let font = crate::theme::font::label();
    let width = crate::renderer::measure_line_width(&text, font, &mut state.text.font_system);
    let monitor = &state.placements[target.monitor_idx];
    let Some(rect) = crate::ui::tooltip::place(
        target,
        width,
        font,
        (monitor.size.0 as f32, monitor.size.1 as f32),
    ) else {
        close(state, dirty);
        return;
    };
    // 3. 【工具提示】【单次显示】登记提示自身区域，消费期限，稳定画面继续使用事件驱动等待
    state.toolbar_tooltip.show(View {
        text,
        rect,
        monitor_idx: target.monitor_idx,
        cache: None,
        theme: None,
    });
    emit_panel_damage(rect, target.monitor_idx, &mut state.damage_rects, dirty);
}
