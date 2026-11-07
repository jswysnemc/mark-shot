use super::{ACTION_ITEMS, ITEMS, Orientation, ToolbarItem};
use crate::theme::{anim, size};
use crate::types::{Outputs, Placement};
use crate::ui::panel::{AnimatedPanel, HoverablePanel, UiPanel};
use std::time::{Duration, Instant};
use tiny_skia::{Pixmap, Rect};

// ==========================================
// 1. UI Layout Constants
// ==========================================
/// Closer than this to the target position counts as arrived.
const POSITION_EPSILON: f32 = 0.5;

// ==========================================
// 2. Component State
// ==========================================
pub struct Toolbar {
    pub toolbar_pixmap: Option<Pixmap>, // to change its opacity (SVG icons limitation)
    pub drawn_look: Option<ToolbarLook>,

    pub secondary: bool,
    pub orientation: Orientation,
    pub cells: Vec<Rect>,
    pub disabled: Vec<usize>,
    pub user_position: Option<(f32, f32)>,
    pub items: &'static [ToolbarItem],
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub opacity: f32,
    pub monitor_idx: usize,

    pub dirty: bool,

    pub selected: Option<usize>,
    pub translating: bool,
    pub hovered: Option<usize>,
    pub to: Outputs,
    pub marker_shape: crate::types::MarkerShape,
    pub marker_filled: bool,

    pub interferes: bool,
    pub last_tick: Option<Instant>,
    pub render_pos: (f32, f32),
    pub placement_kind: Option<ToolbarPlacementKind>,
}

/// everything the toolbar's picture depends on
/// while only its position or opacity changes, the cached picture is reused
#[derive(Clone, PartialEq)]
pub struct ToolbarLook {
    theme: crate::theme::Mode,
    selected: Option<usize>,
    translating: bool,
    hovered: Option<usize>,
    to: Outputs,
    size: (f32, f32),
    marker_shape: crate::types::MarkerShape,
    marker_filled: bool,
    disabled: Vec<usize>,
    orientation: Orientation,
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

impl UiPanel for Toolbar {
    type Item = ToolbarItem;

    fn render_pos(&self) -> (f32, f32) {
        self.render_pos
    }
    fn size(&self) -> (f32, f32) {
        self.size
    }
    fn items(&self) -> &[Self::Item] {
        self.items
    }
    fn padding(&self) -> f32 {
        self.margin()
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

    // Same pattern as SettingsPanel: once fully faded out there's nothing
    // to hit-test or damage, so say so explicitly here instead of every
    // call site re-deriving "invisible" from raw opacity itself.
    fn rect(&self) -> Option<Rect> {
        if self.opacity <= 0.0 {
            return None;
        }
        let (x, y) = self.render_pos();
        let (w, h) = self.size();
        Rect::from_xywh(x, y, w, h)
    }
}

impl HoverablePanel for Toolbar {
    type Hover = Option<usize>;
    fn hovered(&self) -> Self::Hover {
        self.hovered
    }
    fn set_hovered(&mut self, hover: Self::Hover) {
        self.hovered = hover;
    }
}

impl AnimatedPanel for Toolbar {
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
        let target_opacity = if self.interferes { 0.0 } else { 1.0 };
        (self.opacity - target_opacity).abs() > anim::OPACITY_EPSILON
            || self.render_pos != self.position
    }

    fn animate_step(&mut self, dt: f32) -> bool {
        let mut changed = false;

        let animation = &crate::config::get().animation;
        let speed = animation.speed;
        let target_opacity = if self.interferes { 0.0 } else { 1.0 };
        if (self.opacity - target_opacity).abs() > anim::OPACITY_EPSILON {
            let delta = animation.toolbar_fade * speed * dt;
            self.opacity += (target_opacity - self.opacity).signum() * delta;
            self.opacity = self.opacity.clamp(0.0, 1.0);
            changed = true;
        }

        let target = self.position;
        let dx = target.0 - self.render_pos.0;
        let dy = target.1 - self.render_pos.1;
        if dx.abs() > POSITION_EPSILON || dy.abs() > POSITION_EPSILON {
            let t = (animation.toolbar_slide * speed * dt).min(1.0);
            self.render_pos.0 += dx * t;
            self.render_pos.1 += dy * t;
            changed = true;
        } else if self.render_pos != target {
            self.render_pos = target;
            changed = true;
        }

        changed
    }
}

impl Toolbar {
    pub fn new() -> Self {
        let mut toolbar = Self {
            toolbar_pixmap: None,
            drawn_look: None,
            secondary: false,
            orientation: Orientation::Horizontal,
            cells: Vec::new(),
            disabled: Vec::new(),
            user_position: None,
            items: ITEMS,
            size: (0.0, size::panel_height()),
            opacity: 1.0,
            monitor_idx: 0,
            position: (0.0, 0.0),

            dirty: false,
            selected: Some(0),
            translating: false,
            hovered: None,
            to: Outputs::default(),
            marker_shape: crate::types::MarkerShape::default(),
            marker_filled: true,

            interferes: false,
            last_tick: None,
            render_pos: (0.0, 0.0),
            placement_kind: None,
        };

        toolbar.arrange((4096.0, 4096.0));

        toolbar
    }

    /// 【截图工具栏】【副面板构造】返回原版纵向动作工具栏，初始不显示
    pub fn actions() -> Self {
        let mut toolbar = Self::new();
        toolbar.secondary = true;
        toolbar.orientation = Orientation::Vertical;
        toolbar.items = ACTION_ITEMS;
        toolbar.selected = None;
        toolbar.arrange((4096.0, 4096.0));
        toolbar.start_hidden();
        toolbar
    }

    pub fn look(&self) -> ToolbarLook {
        ToolbarLook {
            theme: crate::theme::Mode::current(),
            selected: self.selected,
            translating: self.translating,
            hovered: self.hovered,
            to: self.to,
            size: self.size,
            marker_shape: self.marker_shape,
            marker_filled: self.marker_filled,
            disabled: self.disabled.clone(),
            orientation: self.orientation,
        }
    }

    /// Starts fully faded out, so the first frame doesn't flash it at the corner.
    pub fn start_hidden(&mut self) {
        self.opacity = 0.0;
        self.interferes = true;
        self.placement_kind = Some(ToolbarPlacementKind::Hidden);
    }

    /// hit-tests a local point against the toolbar's current render rect
    /// Returns (is_inside, option_of_button_index).
    pub fn hit_test(&self, local: (f64, f64)) -> (bool, Option<usize>) {
        let Some(rect) = self.rect() else {
            return (false, None);
        };

        let px = local.0 as f32;
        let py = local.1 as f32;

        if py < rect.top() || py > rect.bottom() || px < rect.left() || px > rect.right() {
            return (false, None);
        }

        let point = tiny_skia::Point::from_xy(px - rect.left(), py - rect.top());
        for (idx, cell) in self.cells.iter().enumerate() {
            if super::contains(*cell, (point.x, point.y)) {
                return (
                    true,
                    matches!(self.items[idx], ToolbarItem::Button(_)).then_some(idx),
                );
            }
        }

        (true, None)
    }

    /// Computes the render_pos to start a transition animation from when the
    /// toolbar moves to a new monitor/position. The travel distance is capped
    /// at `max_offset`: on short moves the toolbar starts from its real
    /// current position, on long jumps (e.g. monitor switch) it spawns
    /// `max_offset` pixels away from the target instead of snapping across
    /// the whole screen.
    pub fn compute_transition_start(
        &self,
        old_monitor: usize,
        new_target_local: (f32, f32),
        new_monitor: usize,
        placements: &[Placement],
        max_offset: f32,
    ) -> (f32, f32) {
        let old_p = &placements[old_monitor];
        let new_p = &placements[new_monitor];

        let old_global = (
            old_p.position.0 as f32 + self.render_pos.0,
            old_p.position.1 as f32 + self.render_pos.1,
        );
        let new_global = (
            new_p.position.0 as f32 + new_target_local.0,
            new_p.position.1 as f32 + new_target_local.1,
        );

        let dx = new_global.0 - old_global.0;
        let dy = new_global.1 - old_global.1;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < 0.001 {
            return new_target_local;
        }

        let (ux, uy) = (dx / dist, dy / dist);
        let offset = dist.min(max_offset);

        let start_global = (new_global.0 - ux * offset, new_global.1 - uy * offset);

        (
            start_global.0 - new_p.position.0 as f32,
            start_global.1 - new_p.position.1 as f32,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarPlacementKind {
    Idle,        // no selection: top-center on the cursor's monitor
    Hidden,      // active selection drag: toolbar hidden
    AtSelection, // finished selection: toolbar anchored next to it
}

#[cfg(test)]
mod tests {
    use super::*;

    /// is_animating() used to treat anything within POSITION_EPSILON as finished animation
    /// so toolbar stopped up to 0.5 px of where it had to be.
    ///
    /// it wasn't really visible for me, but on a 2x monitor it's a whole pixel off
    /// and the settings panel (placed from the toolbar's render_pos) shifted with it
    /// and with Shift+arrows the toolbar froze between key presses
    #[test]
    fn toolbar_lands_exactly_on_the_target() {
        for hz in [60.0, 100.0, 144.0] {
            let mut toolbar = Toolbar::new();
            toolbar.position = (800.0, 300.0);
            let mut t = 0.0;
            while toolbar.is_animating() && t < 2.0 {
                toolbar.animate_step(1.0 / hz);
                t += 1.0 / hz;
            }
            assert_eq!(toolbar.render_pos, toolbar.position);
            assert!(t < 1.0, "{hz} Hz took {t} s");
        }
    }
}
