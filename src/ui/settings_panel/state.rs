use crate::editor::EditorState;
use crate::interaction::ScrollAccumulator;
use crate::theme::size;
use crate::types::SpecialKey;
use crate::ui::color_popover::ColorTarget;
use crate::ui::panel::{HoverablePanel, PanelItem, UiPanel};
use crate::ui::text_field::CursorInit;
use crate::ui::text_field::{TextFieldGroup, is_stepper_char};
use tiny_skia::{Pixmap, Rect};

use std::collections::HashMap;

use super::widgets::*;

pub struct SettingsPanel {
    pub widgets: &'static [SettingsWidget],
    pub active_source: Option<SettingsSource>,
    pub position: (f32, f32),
    pub render_pos: (f32, f32),
    pub size: (f32, f32),
    pub monitor_idx: usize,
    pub visible: bool,
    pub dirty: bool,
    pub hovered: Option<usize>,
    pub hovered_arrow: Option<(usize, StepperArrow)>,
    pub selected: Option<usize>,
    pub opacity: f32,
    pub text_background: tiny_skia::Color,
    pub text_layout: Option<crate::ui::text_properties::layout::TextLayout>,
    pub text_points: f32,
    pub text_alpha: i32,
    pub(crate) collection_title: Option<crate::ui::annotation_collection::title::Title>,
    pub panel_pixmap: Option<Pixmap>,
    pub fields: TextFieldGroup<usize>,
    pub arrow_held: Option<ArrowHoldState>,
    pub toggled: HashMap<usize, bool>,
    pub pre_edit_snapshot: Option<crate::editor::history_snapshot::HistorySnapshot>,
    /// % of download progress if any
    pub download: Option<u8>,
    pub marker_shape: crate::types::MarkerShape,
    pub marker_filled: bool,
    pub ruler_mode: crate::features::ruler::RulerMode,
    pub number_style: crate::features::number::NumberStyle,
    pub arrow_style: crate::features::arrow::ArrowStyle,
    pub line_style: crate::features::line::LineStyle,
    pub filter_mode: crate::features::filter::FilterMode,

    /// Scroll accumulator for scrollable fields (hex/rgba)
    /// Keeps scroll fractional state separate from raw events to avoid
    /// processing every single scroll event, trackpad or mouse wheel held
    /// will flood thousands of events that would freeze
    pub scroll: ScrollAccumulator<usize>,
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsPanel {
    pub fn has_outline_swatch(&self) -> bool {
        self.widgets
            .iter()
            .any(|w| matches!(w, SettingsWidget::ColorSwatch(ColorTarget::Outline)))
    }

    pub fn new() -> Self {
        Self {
            widgets: &[],
            active_source: None,
            position: (0.0, 0.0),
            render_pos: (0.0, 0.0),
            size: (0.0, size::panel_height()),
            monitor_idx: 0,
            visible: false,
            dirty: true,
            hovered: None,
            hovered_arrow: None,
            selected: None,
            opacity: 1.0,
            text_background: tiny_skia::Color::TRANSPARENT,
            text_layout: None,
            text_points: 20.0,
            text_alpha: 100,
            collection_title: None,
            panel_pixmap: None,
            fields: TextFieldGroup::new(),
            arrow_held: None,
            toggled: HashMap::new(),
            scroll: ScrollAccumulator::new(),
            pre_edit_snapshot: None,
            download: None,
            marker_shape: crate::types::MarkerShape::default(),
            marker_filled: true,
            ruler_mode: crate::features::ruler::RulerMode::Length,
            number_style: crate::features::number::NumberStyle::Arabic,
            arrow_style: crate::features::arrow::ArrowStyle::Fletched,
            line_style: crate::features::line::LineStyle::Solid,
            filter_mode: crate::features::filter::FilterMode::Pixelate,
        }
    }

    pub fn hit_test(&self, local: (f64, f64)) -> (bool, Option<usize>) {
        let Some(rect) = self.rect() else {
            return (false, None);
        };
        let px = local.0 as f32;
        let py = local.1 as f32;

        if py < rect.top() || py > rect.bottom() || px < rect.left() || px > rect.right() {
            return (false, None);
        }
        if let Some(layout) = self.text_layout {
            return (
                true,
                layout
                    .hit((px - rect.left(), py - rect.top()))
                    .map(|control| control.index()),
            );
        }

        let mut current_x = rect.left() + size::padding();
        for (idx, widget) in self.widgets.iter().enumerate() {
            let w = widget.size();
            let right = current_x + w;
            if px >= current_x && px <= right {
                return if widget.is_button() {
                    (true, Some(idx))
                } else {
                    (true, None)
                };
            }
            current_x += w + widget.trailing_padding();
        }
        (true, None)
    }

    pub fn item_height(&self) -> f32 {
        self.size.1 * crate::config::get().settings_panel.item_height
    }

    /// 【属性面板】【控件矩形】返回 widget_idx 的显示器本地矩形，文字控件使用原版布局，越界返回空值
    pub fn widget_rect(&self, widget_idx: usize) -> Option<Rect> {
        let (x, y, width, height) = self.widget_local_rect(widget_idx)?;
        Rect::from_xywh(x, y, width, height)
    }

    /// 【属性面板】【控件坐标】返回 widget_idx 的位置和尺寸元组，不包含面板外留白
    fn widget_local_rect(&self, widget_idx: usize) -> Option<(f32, f32, f32, f32)> {
        let rect = self.rect()?;
        if let Some(layout) = self.text_layout {
            let control = crate::ui::text_properties::layout::Control::from_index(widget_idx)?;
            let cell = layout.control(control, self.render_pos)?;
            return Some((cell.left(), cell.top(), cell.width(), cell.height()));
        }
        let h = self.size.1;
        let item_h = self.item_height();
        let item_y = rect.top() + (h - item_h) / 2.0;

        let mut current_x = rect.left() + size::padding();
        for (idx, widget) in self.widgets.iter().enumerate() {
            let w = widget.size();
            if idx == widget_idx {
                return Some((current_x, item_y, w, item_h));
            }
            current_x += w + widget.trailing_padding();
        }
        None
    }

    pub fn stepper_arrow_hit(&self, widget_idx: usize, local: (f64, f64)) -> Option<StepperArrow> {
        if !matches!(
            self.widgets.get(widget_idx),
            Some(SettingsWidget::Stepper { .. })
        ) {
            return None;
        }
        let (item_x, item_y, item_w, item_h) = self.widget_local_rect(widget_idx)?;

        let px = local.0 as f32;
        let py = local.1 as f32;

        let zone_left = item_x + item_w - crate::config::get().settings_panel.stepper_arrow_zone;
        if px < zone_left || px > item_x + item_w || py < item_y || py > item_y + item_h {
            return None;
        }

        let mid_y = item_y + item_h / 2.0;
        Some(if py < mid_y {
            StepperArrow::Up
        } else {
            StepperArrow::Down
        })
    }

    // ── editing input fields ─────────────────────────────────────────
    // wrapper around TextFieldGroup: same external signature, but with dirty flag management
    pub fn begin_edit(&mut self, widget_idx: usize, initial_text: String, cursor: CursorInit) {
        self.fields.begin_edit(widget_idx, initial_text, cursor);
        self.dirty = true;
    }

    pub fn commit_edit(&mut self) -> Option<(usize, String)> {
        let result = self.fields.commit_edit();
        if result.is_some() {
            self.dirty = true;
        }
        result
    }

    pub fn is_editing(&self) -> bool {
        self.fields.is_editing()
    }

    pub fn insert_char(&mut self, ch: char) -> bool {
        let inserted = self.fields.insert_char(ch, is_stepper_char);
        if inserted {
            self.dirty = true;
        }
        inserted
    }

    pub fn handle_key(&mut self, key: SpecialKey, ctrl: bool, shift: bool) -> (bool, bool) {
        let result = self.fields.handle_key(key, ctrl, shift, is_stepper_char);
        if result.0 {
            self.dirty = true;
        }
        result
    }

    pub fn sync_value(&mut self, idx: usize, text: String) {
        self.fields.sync_value(idx, text);
    }

    pub fn widget_text_x(&self, widget_idx: usize) -> Option<f32> {
        let rect = self.rect()?;
        let mut current_x = rect.left() + size::padding();
        for (idx, widget) in self.widgets.iter().enumerate() {
            if idx == widget_idx {
                return Some(current_x + size::padding());
            }
            current_x += widget.size() + widget.trailing_padding();
        }
        None
    }

    pub fn is_toggled(&self, idx: usize) -> bool {
        self.toggled.get(&idx).copied().unwrap_or(false)
    }
    pub fn set_toggled(&mut self, idx: usize, value: bool) {
        self.toggled.insert(idx, value);
        self.dirty = true;
    }
    pub fn toggle(&mut self, idx: usize) -> bool {
        let v = !self.is_toggled(idx);
        self.set_toggled(idx, v);
        v
    }

    /// Accumulate a fractional scroll `delta` (already in "steps", not raw
    /// pixels) for the given field and return how many whole steps have
    /// accumulated since the last call (may be negative). Switching fields
    /// discards the leftover from the previous one.
    ///
    /// Delegates to [`ScrollAccumulator`], see its docs for the two
    /// flood-prevention safeguards (rate limit + step cap)
    pub fn scroll_step(&mut self, widget_idx: usize, delta: f32) -> i32 {
        self.scroll.step(widget_idx, delta)
    }

    /// scroll reset. Call on any non-scroll user action
    /// (click, key press) so stale queued events don't trigger afterwards
    pub fn cancel_scroll(&mut self) {
        self.scroll.cancel();
    }
}

impl UiPanel for SettingsPanel {
    type Item = SettingsWidget;

    fn render_pos(&self) -> (f32, f32) {
        self.render_pos
    }
    fn size(&self) -> (f32, f32) {
        self.size
    }
    fn items(&self) -> &[Self::Item] {
        self.widgets
    }
    fn padding(&self) -> f32 {
        size::padding()
    }
    fn monitor_idx(&self) -> usize {
        self.monitor_idx
    }
    fn set_dirty(&mut self) {
        self.dirty = true;
    }
    fn is_dirty(&self) -> bool {
        self.dirty
    }
    fn is_visible(&self) -> bool {
        self.visible
    }

    fn rect(&self) -> Option<Rect> {
        if !self.visible {
            return None;
        }
        let (x, y) = self.render_pos();
        let (w, h) = self.size();
        Rect::from_xywh(x, y, w, h)
    }
}

impl HoverablePanel for SettingsPanel {
    type Hover = (Option<usize>, Option<(usize, StepperArrow)>);

    fn hovered(&self) -> Self::Hover {
        (self.hovered, self.hovered_arrow)
    }
    fn set_hovered(&mut self, hover: Self::Hover) {
        self.hovered = hover.0;
        self.hovered_arrow = hover.1;
    }
}

// ── Placement ─────────────────────────────────────────

pub fn compute_settings_placement(editor_state: &EditorState) -> ((f32, f32), usize) {
    let tb = &editor_state.toolbar;
    let (panel_w, panel_h) = editor_state.settings_panel.size;
    let bounds = editor_state.placements[tb.monitor_idx].size;
    let margin = 8.0;
    let (render_x, render_y) = tb.render_pos;
    let (x, y) = if tb.orientation == crate::ui::toolbar::Orientation::Vertical {
        let right = render_x + tb.size.0 + 9.0;
        (
            if right + panel_w <= bounds.0 as f32 - margin {
                right
            } else {
                render_x - panel_w - 10.0
            },
            render_y,
        )
    } else {
        let below = render_y + tb.size.1 + 9.0;
        (
            if render_x + panel_w > bounds.0 as f32 - margin {
                render_x + tb.size.0 - 1.0 - panel_w
            } else {
                render_x
            },
            if below + panel_h <= bounds.1 as f32 - margin {
                below
            } else {
                render_y - panel_h - 10.0
            },
        )
    };
    (
        (
            x.clamp(margin, (bounds.0 as f32 - panel_w - margin).max(margin)),
            y.clamp(margin, (bounds.1 as f32 - panel_h - margin).max(margin)),
        ),
        tb.monitor_idx,
    )
}
