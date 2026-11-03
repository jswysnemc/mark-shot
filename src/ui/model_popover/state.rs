// list of OCR models: select, download, cancel, delete

use std::time::{Duration, Instant};
use tiny_skia::{Pixmap, Rect};

use crate::config::ModelPicker;
use crate::ocr::models::{MODELS, ModelStatus};
use crate::theme::{anim, font};
use crate::ui::panel::{AnimatedPanel, HoverablePanel, PanelItem, UiPanel};

pub fn cfg() -> &'static ModelPicker {
    &crate::config::get().model_picker
}

pub fn note_font_size() -> f32 {
    font::small()
}

pub fn height() -> f32 {
    let c = cfg();
    c.padding * 2.0 + c.title_height + c.row_height * MODELS.len() as f32
}

#[derive(Debug, Clone, Copy)]
pub enum ModelPopoverItem {}

impl PanelItem for ModelPopoverItem {
    fn size(&self) -> f32 {
        match *self {}
    }
    fn trailing_padding(&self) -> f32 {
        match *self {}
    }
    fn is_button(&self) -> bool {
        match *self {}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelPopoverElement {
    Row(usize),
    Button(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelRow {
    pub status: ModelStatus,
    pub active: bool,
    pub size: u64,
    pub recommended: bool,
}

pub fn row_geom(origin: (f32, f32), idx: usize) -> Option<Rect> {
    let c = cfg();
    Rect::from_xywh(
        origin.0 + c.padding,
        origin.1 + c.padding + c.title_height + idx as f32 * c.row_height,
        c.width - c.padding * 2.0,
        c.row_height,
    )
}

pub fn button_geom(origin: (f32, f32), idx: usize) -> Option<Rect> {
    let c = cfg();
    let row = row_geom(origin, idx)?;
    Rect::from_xywh(
        row.right() - c.row_padding / 2.0 - c.button_size,
        row.top() + (c.row_height - c.button_size) / 2.0,
        c.button_size,
        c.button_size,
    )
}

fn point_in_rect(local: (f64, f64), rect: Rect) -> bool {
    let px = local.0 as f32;
    let py = local.1 as f32;
    px >= rect.left() && px <= rect.right() && py >= rect.top() && py <= rect.bottom()
}

pub struct ModelPopover {
    pub pixmap: Option<Pixmap>,

    pub position: (f32, f32),
    pub render_pos: (f32, f32),
    pub size: (f32, f32),
    pub opacity: f32,
    pub monitor_idx: usize,

    pub open: bool,
    pub dirty: bool,

    pub hovered: Option<ModelPopoverElement>,
    pub last_tick: Option<Instant>,

    pub rows: Vec<ModelRow>,
}

impl Default for ModelPopover {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelPopover {
    pub fn new() -> Self {
        Self {
            pixmap: None,
            position: (0.0, 0.0),
            render_pos: (0.0, 0.0),
            size: (cfg().width, height()),
            opacity: 0.0,
            monitor_idx: 0,
            open: false,
            dirty: false,
            hovered: None,
            last_tick: None,
            rows: Vec::with_capacity(MODELS.len()),
        }
    }

    pub fn hit_test(&self, local: (f64, f64)) -> bool {
        self.rect().is_some_and(|rect| point_in_rect(local, rect))
    }

    pub fn element_at(&self, local: (f64, f64)) -> Option<ModelPopoverElement> {
        let rect = self.rect()?;
        let origin = (rect.left(), rect.top());
        (0..MODELS.len()).find_map(|idx| {
            let row = row_geom(origin, idx)?;
            if !point_in_rect(local, row) {
                return None;
            }
            let on_button = button_geom(origin, idx).is_some_and(|b| point_in_rect(local, b));
            Some(if on_button {
                ModelPopoverElement::Button(idx)
            } else {
                ModelPopoverElement::Row(idx)
            })
        })
    }

    /// true if something chnaged
    pub fn sync_rows(&mut self, rows: impl Iterator<Item = ModelRow>) -> bool {
        let mut changed = false;
        for (idx, row) in rows.enumerate() {
            match self.rows.get_mut(idx) {
                Some(old) if *old == row => {}
                Some(old) => {
                    *old = row;
                    changed = true;
                }
                None => {
                    self.rows.push(row);
                    changed = true;
                }
            }
        }
        changed
    }
}

impl UiPanel for ModelPopover {
    type Item = ModelPopoverItem;

    fn render_pos(&self) -> (f32, f32) {
        self.render_pos
    }
    fn size(&self) -> (f32, f32) {
        self.size
    }
    fn items(&self) -> &[Self::Item] {
        &[]
    }
    fn padding(&self) -> f32 {
        cfg().padding
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
        self.open || self.opacity > 0.0
    }

    fn rect(&self) -> Option<Rect> {
        if self.opacity <= 0.0 {
            return None;
        }
        let (x, y) = self.render_pos();
        let (w, h) = self.size();
        Rect::from_xywh(x, y, w, h)
    }
}

impl HoverablePanel for ModelPopover {
    type Hover = Option<ModelPopoverElement>;
    fn hovered(&self) -> Self::Hover {
        self.hovered
    }
    fn set_hovered(&mut self, hover: Self::Hover) {
        self.hovered = hover;
    }
}

impl AnimatedPanel for ModelPopover {
    fn last_tick(&self) -> Option<Instant> {
        self.last_tick
    }
    fn set_last_tick(&mut self, at: Option<Instant>) {
        self.last_tick = at;
    }

    fn anim_interval(&self) -> Duration {
        anim::frame()
    }
    fn anim_dt(&self) -> f32 {
        anim::dt()
    }

    fn is_animating(&self) -> bool {
        let target = if self.open { 1.0 } else { 0.0 };
        (self.opacity - target).abs() > anim::OPACITY_EPSILON
    }

    fn animate_step(&mut self, dt: f32) -> bool {
        let target = if self.open { 1.0 } else { 0.0 };
        if (self.opacity - target).abs() > anim::OPACITY_EPSILON {
            let delta = anim::popover_fade() * crate::config::get().animation.speed * dt;
            self.opacity += (target - self.opacity).signum() * delta;
            self.opacity = self.opacity.clamp(0.0, 1.0);
            true
        } else {
            false
        }
    }
}
