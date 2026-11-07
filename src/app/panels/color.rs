mod fields;
use super::settings::swatch_color;
use crate::editor::EditorState;
use crate::editor::edits::{apply_color_selection, damage_color_popover, use_color};
use crate::ui::color_popover;
use crate::ui::color_popover::{ColorField, ColorPopoverElement, ColorTarget};
use crate::ui::panel::{UiPanel, sync_panel_hover, sync_panel_rect};
pub use fields::*;

/// 【属性取色】【面板同步】按 editor_state 当前模式定位并同步控件；dirty_mask 返回显示器损坏区域
pub fn update_color_popover(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    let old_rect = editor_state.color_popover.rect();
    let old_monitor = editor_state.color_popover.monitor_idx;

    // 1. 【属性取色】【独立锚点】快捷色板固定在指针处，属性取色器跟随对应颜色按钮而不是旧工具栏侧边
    let radial = editor_state.color_popover.radial.is_some();
    let mut visual_changed = false;
    if radial {
        visual_changed = super::radial_palette::update(editor_state);
    } else if editor_state.color_popover.is_visible() || editor_state.color_popover.open {
        let panel = &editor_state.settings_panel;
        let monitor = panel.monitor_idx;
        let bounds = editor_state.placements[monitor].size;
        let size = (
            color_popover::width().min((bounds.0 as f32 - 16.0).max(160.0)),
            color_popover::height().min((bounds.1 as f32 - 16.0).max(180.0)),
        );
        let properties = panel.rect().unwrap_or_else(|| {
            tiny_skia::Rect::from_xywh(
                panel.render_pos.0,
                panel.render_pos.1,
                panel.size.0.max(1.0),
                panel.size.1.max(1.0),
            )
            .unwrap()
        });
        let mut anchor = (
            properties.left() + ((properties.width() - 1.0) / 2.0).floor(),
            properties.bottom() - 1.0,
        );
        for (index, item) in panel.widgets.iter().enumerate() {
            if item.color_target() == Some(editor_state.color_popover.target)
                && let Some(cell) = panel.widget_rect(index)
            {
                anchor = (
                    cell.left() + ((cell.width() - 1.0) / 2.0).floor(),
                    cell.top() + ((cell.height() - 1.0) / 2.0).floor(),
                );
                break;
            }
        }
        let position = color_popover::layout::position(anchor, properties, size, bounds);
        editor_state.color_popover.size = size;
        editor_state.color_popover.position = position;
        editor_state.color_popover.render_pos = position;
        editor_state.color_popover.monitor_idx = monitor;
    }
    editor_state.color_popover.opacity = if editor_state.color_popover.open {
        1.0
    } else {
        0.0
    };

    editor_state.color_popover.sync_field_values();

    let picking = editor_state.pick_once;
    let picking_changed = editor_state.color_popover.picking != picking;
    editor_state.color_popover.picking = picking;

    sync_panel_rect(
        &mut editor_state.color_popover,
        old_rect,
        old_monitor,
        picking_changed || visual_changed,
        &mut editor_state.damage_rects,
        dirty_mask,
    );

    if editor_state.color_popover.rect().is_some() {
        let hovered = hover_test(editor_state, editor_state.input.pointer.local);
        sync_panel_hover(
            &mut editor_state.color_popover,
            hovered,
            &mut editor_state.damage_rects,
            dirty_mask,
        );
    }
}

/// 【属性取色】【悬停定位】返回 editor_state 中 local 对应控件，快捷色板与属性取色器独立命中
fn hover_test(editor_state: &EditorState, local: (f64, f64)) -> Option<ColorPopoverElement> {
    let cp = &editor_state.color_popover;

    if let Some(radial) = &cp.radial {
        return radial.hovered.map(ColorPopoverElement::Swatch);
    }
    if cp.alpha_slider_hit(local) {
        return Some(ColorPopoverElement::AlphaSlider);
    }
    if cp.sv_square_hit(local) {
        return Some(ColorPopoverElement::SvSquare);
    }
    if cp.hue_slider_hit(local) {
        return Some(ColorPopoverElement::HueSlider);
    }
    if let Some(idx) = cp.swatch_hit(local) {
        return Some(ColorPopoverElement::Swatch(idx));
    }
    if cp.eyedropper_hit(local) {
        return Some(ColorPopoverElement::Eyedropper);
    }
    if cp.hex_field_hit(local) {
        return Some(ColorPopoverElement::Field(ColorField::Hex));
    }
    if let Some(field) = cp.rgba_field_hit(local) {
        return Some(ColorPopoverElement::Field(field));
    }
    None
}

/// 【属性取色】【左键按下】按 editor_state 当前指针开始轨道手势或字段编辑；dirty_mask 返回重绘区域
pub fn handle_color_popover_click(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    let local = editor_state.input.pointer.local;

    if super::radial_palette::press(editor_state) {
        return;
    }
    if editor_state.color_popover.alpha_slider_hit(local) {
        editor_state.color_popover.alpha_dragging = true;
        editor_state.color_popover.set_alpha_from_local(local);
        editor_state.color_popover.sync_field_values();
        damage_color_popover(editor_state, dirty_mask);
        let color = editor_state.color_popover.sv_square.color();
        apply_color_selection(editor_state, color, false, dirty_mask);
        return;
    }
    if editor_state.pick_once && !editor_state.color_popover.eyedropper_hit(local) {
        crate::tools::eyedropper::end_pick_once(editor_state, dirty_mask);
    }

    if editor_state.color_popover.sv_square_hit(local) {
        editor_state.color_popover.sv_square.dragging = true;
        editor_state.color_popover.set_sv_from_local(local);
        editor_state.color_popover.sync_field_values();
        damage_color_popover(editor_state, dirty_mask);

        let color = editor_state.color_popover.sv_square.color();
        apply_color_selection(editor_state, color, false, dirty_mask);
        return;
    }

    if editor_state.color_popover.hue_slider_hit(local) {
        editor_state.color_popover.hue_dragging = true;
        editor_state.color_popover.set_hue_from_local(local);
        editor_state.color_popover.sync_field_values();
        damage_color_popover(editor_state, dirty_mask);

        let color = editor_state.color_popover.sv_square.color();
        apply_color_selection(editor_state, color, false, dirty_mask);
        return;
    }

    if editor_state.color_popover.eyedropper_hit(local) {
        crate::tools::eyedropper::toggle_pick_once(editor_state, dirty_mask);
        return;
    }

    if let Some(idx) = editor_state.color_popover.swatch_hit(local) {
        if let Some(color) = editor_state.color_popover.palette().get(idx).copied() {
            use_color(editor_state, color, dirty_mask);
        }
        return;
    }

    if editor_state.color_popover.hex_field_hit(local) {
        begin_color_field_edit(editor_state, ColorField::Hex, local, dirty_mask);
        return;
    }

    if let Some(field) = editor_state.color_popover.rgba_field_hit(local) {
        begin_color_field_edit(editor_state, field, local, dirty_mask);
        return;
    }

    update_color_popover(editor_state, dirty_mask);
}

/// 【属性取色】【轨道拖动】用 editor_state 实际指针继续已开始的左键手势；dirty_mask 返回重绘区域
pub fn handle_color_popover_drag(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    let placement = &editor_state.placements[editor_state.color_popover.monitor_idx];
    // 1. 【属性取色】【跨屏坐标】拖动抓取仍属于原面板，不把另一显示器的本地横坐标作为滑条输入
    let local = (
        editor_state.input.pointer.global.0 - f64::from(placement.position.0),
        editor_state.input.pointer.global.1 - f64::from(placement.position.1),
    );

    let changed = if editor_state.color_popover.sv_square.dragging {
        editor_state.color_popover.set_sv_from_local(local);
        true
    } else if editor_state.color_popover.hue_dragging {
        editor_state.color_popover.set_hue_from_local(local);
        true
    } else if editor_state.color_popover.alpha_dragging {
        editor_state.color_popover.set_alpha_from_local(local);
        true
    } else {
        false
    };

    if !changed {
        return;
    }

    editor_state.color_popover.sync_field_values();
    damage_color_popover(editor_state, dirty_mask);

    let color = editor_state.color_popover.sv_square.color();
    apply_color_selection(editor_state, color, false, dirty_mask);
}

/// 【属性取色】【左键释放】结束 editor_state 的轨道手势并记录最终色；dirty_mask 返回重绘区域，保留整次打开的快照
pub fn handle_color_popover_release(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    if super::radial_palette::release(editor_state, dirty_mask) {
        return;
    }
    let was_dragging = editor_state.color_popover.sv_square.dragging
        || editor_state.color_popover.hue_dragging
        || editor_state.color_popover.alpha_dragging;

    editor_state.color_popover.sv_square.dragging = false;
    editor_state.color_popover.hue_dragging = false;
    editor_state.color_popover.alpha_dragging = false;

    if !was_dragging {
        return;
    }

    let color = editor_state.color_popover.sv_square.color();
    editor_state.color_popover.record_used_color(color);
    damage_color_popover(editor_state, dirty_mask);
}

/// 【属性取色】【目标切换】按 target 读取 editor_state 属性对象颜色；dirty_mask 返回重绘区域，不覆盖默认色
pub fn switch_color_popover_to(
    editor_state: &mut EditorState,
    target: ColorTarget,
    dirty_mask: &mut u32,
) {
    super::line::close(editor_state, dirty_mask);
    super::text::font_popup::close(editor_state, dirty_mask);
    super::filter::close(editor_state, dirty_mask);
    if editor_state.color_popover.fields.is_editing() {
        commit_color_field_edit(editor_state, dirty_mask);
    }
    let color = swatch_color(editor_state, target);
    editor_state.color_popover.target = target;
    editor_state.color_popover.select_color(color);
    editor_state.color_popover.refresh_palette();
    editor_state.color_popover.sync_field_values();
    damage_color_popover(editor_state, dirty_mask);
}

/// 【属性取色】【面板关闭】提交 editor_state 整次打开的颜色事务并取消手势；dirty_mask 返回旧面板损坏区域
pub fn close_color_popover(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    if let Some(radial) = &mut editor_state.color_popover.radial {
        radial.armed = None;
    } else {
        handle_color_popover_release(editor_state, dirty_mask);
    }
    if editor_state.color_popover.fields.is_editing() {
        commit_color_field_edit(editor_state, dirty_mask);
    }
    let snapshot = editor_state.color_popover.take_edit_snapshot();
    editor_state.commit_snapshot(snapshot);
    editor_state.color_popover.open = false;
    editor_state.settings_panel.selected = None;
    update_color_popover(editor_state, dirty_mask);
}

/// 【属性取色】【按钮切换】按 target 打开或关闭独立取色器；dirty 返回变化，快捷色板不冒充属性弹层
pub fn toggle(editor_state: &mut EditorState, target: ColorTarget, dirty: &mut u32) {
    if editor_state.color_popover.open
        && editor_state.color_popover.radial.is_none()
        && editor_state.color_popover.target == target
    {
        close_color_popover(editor_state, dirty);
        return;
    }
    close_color_popover(editor_state, dirty);
    editor_state.color_popover.radial = None;
    switch_color_popover_to(editor_state, target, dirty);
    editor_state.color_popover.open = true;
    update_color_popover(editor_state, dirty);
}
