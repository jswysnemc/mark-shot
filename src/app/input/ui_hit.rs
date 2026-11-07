//! 【界面】【命中与光标】只判断统一弹层、按钮和当前工具，不修改编辑状态

use crate::editor::EditorState;
use crate::tools::dispatch_cursor;
use crate::types::CursorIcon;
use crate::ui::line_properties::LineChoice;

pub(in crate::app) enum UiHit {
    FontPanelInside,
    FontPanelOutside,
    FilterPopoverInside,
    FilterPopoverOutside,
    ColorPopoverInside,
    ColorPopoverOutside,
    ModelPopoverInside,
    ModelPopoverOutside,
    MarkerPopoverInside,
    MarkerPopoverOutside,
    RulerPopoverInside,
    RulerPopoverOutside,
    NumberPopoverInside,
    NumberPopoverOutside,
    LinePopoverInside(LineChoice),
    LinePopoverOutside(LineChoice),
    ToolbarItem(usize),
    ToolbarBackground,
    ActionToolbarItem(usize),
    ActionToolbarBackground,
    SettingsItem(usize),
    SettingsBackground,
    None,
}

/// 【界面】【界面命中】按弹层优先级检查 editor_state 中显示器本地 local；返回命中类别
pub(in crate::app) fn hit_test_ui(editor_state: &EditorState, local: (f64, f64)) -> UiHit {
    if editor_state.font_panel.open {
        return if editor_state.input.pointer.monitor_idx == editor_state.font_panel.monitor_idx
            && editor_state.font_panel.hit_test(local)
        {
            UiHit::FontPanelInside
        } else {
            UiHit::FontPanelOutside
        };
    }
    if editor_state.filter_popover.open {
        return if editor_state.input.pointer.monitor_idx == editor_state.filter_popover.monitor_idx
            && editor_state.filter_popover.hit_test(local)
        {
            UiHit::FilterPopoverInside
        } else {
            UiHit::FilterPopoverOutside
        };
    }
    for kind in LineChoice::ALL {
        let panel = super::super::panels::line::popup(editor_state, kind);
        if panel.open {
            return if editor_state.input.pointer.monitor_idx == panel.monitor_idx
                && panel.hit_test(local)
            {
                UiHit::LinePopoverInside(kind)
            } else {
                UiHit::LinePopoverOutside(kind)
            };
        }
    }
    if editor_state.number_popover.open {
        return if editor_state.input.pointer.monitor_idx == editor_state.number_popover.monitor_idx
            && editor_state.number_popover.hit_test(local)
        {
            UiHit::NumberPopoverInside
        } else {
            UiHit::NumberPopoverOutside
        };
    }
    if editor_state.ruler_popover.open {
        return if editor_state.input.pointer.monitor_idx == editor_state.ruler_popover.monitor_idx
            && editor_state.ruler_popover.hit_test(local)
        {
            UiHit::RulerPopoverInside
        } else {
            UiHit::RulerPopoverOutside
        };
    }
    // 1. 【界面】【界面命中】打开的弹层优先接收点击
    if editor_state.marker_popover.open {
        return if editor_state.input.pointer.monitor_idx == editor_state.marker_popover.monitor_idx
            && editor_state.marker_popover.hit_test(local)
        {
            UiHit::MarkerPopoverInside
        } else {
            UiHit::MarkerPopoverOutside
        };
    }

    if editor_state.color_popover.open {
        return if editor_state.input.pointer.monitor_idx == editor_state.color_popover.monitor_idx
            && editor_state.color_popover.hit_test(local)
        {
            UiHit::ColorPopoverInside
        } else {
            UiHit::ColorPopoverOutside
        };
    }

    if editor_state.model_popover.open {
        return if editor_state.input.pointer.monitor_idx == editor_state.model_popover.monitor_idx
            && editor_state.model_popover.hit_test(local)
        {
            UiHit::ModelPopoverInside
        } else {
            UiHit::ModelPopoverOutside
        };
    }

    // 2. 【界面】【界面命中】工具栏按钮与背景分开处理
    let (in_toolbar, tb_item) =
        if editor_state.input.pointer.monitor_idx == editor_state.toolbar.monitor_idx {
            editor_state.toolbar.hit_test(local)
        } else {
            (false, None)
        };
    if in_toolbar {
        return match tb_item {
            Some(idx) => UiHit::ToolbarItem(idx),
            None => UiHit::ToolbarBackground,
        };
    }

    if editor_state.input.pointer.monitor_idx == editor_state.action_toolbar.monitor_idx {
        let (inside, item) = editor_state.action_toolbar.hit_test(local);
        if inside {
            return item.map_or(UiHit::ActionToolbarBackground, UiHit::ActionToolbarItem);
        }
    }

    // 3. 【界面】【界面命中】属性控件不把点击传给画布
    if editor_state.settings_panel.visible
        && editor_state.input.pointer.monitor_idx == editor_state.settings_panel.monitor_idx
    {
        let (in_settings, widget_idx) = editor_state.settings_panel.hit_test(local);
        if in_settings {
            return match widget_idx {
                Some(idx) => UiHit::SettingsItem(idx),
                None => UiHit::SettingsBackground,
            };
        }
    }

    UiHit::None
}

/// 【界面】【光标选择】依据 editor_state 的弹层、按钮和工具状态返回光标类型
pub fn compute_cursor(editor_state: &EditorState) -> CursorIcon {
    if editor_state.upload.busy() || editor_state.selection_translation.busy() {
        return CursorIcon::Progress;
    }
    if editor_state.selection_translation.popup.is_some() {
        return CursorIcon::Pointer;
    }
    if editor_state.selection_size.popup.is_some() {
        return CursorIcon::Pointer;
    }
    if let Some(bar) = &editor_state.selection_translation.bar
        && bar.monitor == editor_state.input.pointer.monitor_idx
        && crate::ui::selection_translation::draw::contains(
            bar.rect,
            editor_state.input.pointer.local,
        )
    {
        return if bar.hit(editor_state.input.pointer.local).is_some() {
            CursorIcon::Pointer
        } else {
            CursorIcon::Default
        };
    }
    if editor_state.external_panel.kind.is_some() {
        return if editor_state
            .external_panel
            .hit(
                editor_state.input.pointer.monitor_idx,
                editor_state.input.pointer.local,
            )
            .is_some()
        {
            CursorIcon::Pointer
        } else {
            CursorIcon::Default
        };
    }
    if editor_state.text_context_menu.is_some() {
        return CursorIcon::Default;
    }
    if let Some(hit) = super::super::selection_size::input::hit(editor_state) {
        return match hit {
            crate::ui::selection_size::state::Hit::Field(_) => CursorIcon::Text,
            crate::ui::selection_size::state::Hit::Lock
            | crate::ui::selection_size::state::Hit::Ratio => CursorIcon::Pointer,
            crate::ui::selection_size::state::Hit::Background => CursorIcon::Default,
        };
    }
    if super::super::canvas_navigation::input::owns_pointer(editor_state) {
        return if editor_state.navigation.pan.is_some() {
            CursorIcon::Grabbing
        } else {
            CursorIcon::Default
        };
    }
    let picker = &editor_state.color_popover;
    if picker.open && picker.sv_square.dragging {
        return CursorIcon::Crosshair;
    }
    if picker.open && (picker.hue_dragging || picker.alpha_dragging) {
        return CursorIcon::Pointer;
    }
    match hit_test_ui(editor_state, editor_state.input.pointer.local) {
        UiHit::FontPanelInside => match editor_state
            .font_panel
            .hit(editor_state.input.pointer.local)
        {
            Some(
                crate::ui::font_panel::state::Hit::Search | crate::ui::font_panel::state::Hit::Size,
            ) => CursorIcon::Text,
            Some(_) => CursorIcon::Pointer,
            None => CursorIcon::Default,
        },
        UiHit::FontPanelOutside => CursorIcon::Default,
        UiHit::ColorPopoverInside => {
            let point = editor_state.input.pointer.local;
            if picker.radial.is_some() {
                CursorIcon::Default
            } else if picker.sv_square_hit(point) {
                CursorIcon::Crosshair
            } else if picker.hex_field_hit(point) {
                CursorIcon::Text
            } else if picker.hue_slider_hit(point)
                || picker.alpha_slider_hit(point)
                || picker.swatch_hit(point).is_some()
            {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            }
        }
        UiHit::ToolbarItem(_) | UiHit::ActionToolbarItem(_) | UiHit::SettingsItem(_) => {
            CursorIcon::Pointer
        }
        UiHit::ModelPopoverInside => {
            if editor_state
                .model_popover
                .element_at(editor_state.input.pointer.local)
                .is_some()
            {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            }
        }
        UiHit::FilterPopoverInside => {
            if editor_state
                .filter_popover
                .item_at(editor_state.input.pointer.local)
                .is_some()
            {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            }
        }
        UiHit::MarkerPopoverInside => {
            if editor_state
                .marker_popover
                .item_at(editor_state.input.pointer.local)
                .is_some()
            {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            }
        }
        UiHit::RulerPopoverInside => {
            if editor_state
                .ruler_popover
                .item_at(editor_state.input.pointer.local)
                .is_some()
            {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            }
        }
        UiHit::NumberPopoverInside => {
            if editor_state
                .number_popover
                .item_at(editor_state.input.pointer.local)
                .is_some()
            {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            }
        }
        UiHit::LinePopoverInside(kind) => {
            if super::super::panels::line::popup(editor_state, kind)
                .item_at(editor_state.input.pointer.local)
                .is_some()
            {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            }
        }
        UiHit::ToolbarBackground | UiHit::ActionToolbarBackground | UiHit::SettingsBackground => {
            CursorIcon::Default
        }
        UiHit::ColorPopoverOutside
        | UiHit::FilterPopoverOutside
        | UiHit::ModelPopoverOutside
        | UiHit::MarkerPopoverOutside
        | UiHit::RulerPopoverOutside
        | UiHit::NumberPopoverOutside
        | UiHit::LinePopoverOutside(_)
        | UiHit::None => {
            if editor_state.pick_once {
                CursorIcon::Crosshair
            } else if crate::tools::text::session_input::inside(editor_state) {
                CursorIcon::Text
            } else {
                dispatch_cursor(editor_state.selected_tool, editor_state)
            }
        }
    }
}
