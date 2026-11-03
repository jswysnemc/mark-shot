use std::time::{Duration, Instant};
use tiny_skia::Rect;

use crate::editor::DamageZone;
use crate::editor::dirty::mark_dirty;
use crate::theme::anim;
use crate::utils::rects_overlap;

pub trait PanelItem {
    fn size(&self) -> f32;
    fn trailing_padding(&self) -> f32;
    fn is_button(&self) -> bool;
}

pub trait UiPanel {
    type Item: PanelItem;

    fn render_pos(&self) -> (f32, f32);
    fn size(&self) -> (f32, f32);
    fn items(&self) -> &[Self::Item];
    fn padding(&self) -> f32;

    /// Which monitor this panel is currently placed on. Every real panel
    /// in this app is monitor-anchored
    fn monitor_idx(&self) -> usize;

    fn set_dirty(&mut self);
    fn is_dirty(&self) -> bool;

    /// Panels that come and go override this; the toolbar is always up.
    fn is_visible(&self) -> bool {
        true
    }

    /// Panel's rect right now, or None if there's nothing to show/
    fn rect(&self) -> Option<Rect> {
        let (x, y) = self.render_pos();
        let (w, h) = self.size();
        Rect::from_xywh(x, y, w, h)
    }

    fn width(&self) -> f32 {
        let mut total = self.padding() * 2.0;
        for item in self.items() {
            total += item.size() + item.trailing_padding();
        }
        total
    }
}

// ==========================================
// Shared damage/dirty primitives
// ==========================================
//
pub fn emit_panel_damage(
    rect: Rect,
    monitor_idx: usize,
    damage_rects: &mut Vec<DamageZone>,
    dirty_mask: &mut u32,
) {
    damage_rects.push(DamageZone::Local { monitor_idx, rect });
    mark_dirty(dirty_mask, monitor_idx);
}

/// Picks the monitor's panel for rendering and marks it dirty if damaged by now
pub fn panel_to_draw<'p, P: UiPanel>(
    panel: &'p mut P,
    monitor_idx: usize,
    dirty_rect: Option<&Rect>,
) -> Option<&'p mut P> {
    if panel.monitor_idx() != monitor_idx || !panel.is_visible() {
        return None;
    }
    if !panel.is_dirty()
        && let Some(rect) = panel.rect()
        && dirty_rect.is_none_or(|dirty| rects_overlap(dirty, &rect))
    {
        panel.set_dirty();
    }
    panel.is_dirty().then_some(panel)
}

pub fn sync_panel_rect<P: UiPanel>(
    panel: &mut P,
    old_rect: Option<Rect>,
    old_monitor: usize,
    extra_changed: bool,
    damage_rects: &mut Vec<DamageZone>,
    dirty_mask: &mut u32,
) -> bool {
    let new_rect = panel.rect();
    let new_monitor = panel.monitor_idx();

    let changed = extra_changed || old_rect != new_rect || old_monitor != new_monitor;
    if !changed {
        return false;
    }

    panel.set_dirty();
    if let Some(rect) = old_rect {
        emit_panel_damage(rect, old_monitor, damage_rects, dirty_mask);
    }
    if let Some(rect) = new_rect {
        emit_panel_damage(rect, new_monitor, damage_rects, dirty_mask);
    }
    true
}

/// A panel that tracks what's hovered.
/// Toolbar's hover is a button index; SettingsPanel's is a
/// button index and a stepper arrow, both fit as 'Self::Hover', so
/// 'sync_panel_hover' works for anything without caring what type it is
pub trait HoverablePanel: UiPanel {
    type Hover: PartialEq + Copy;
    fn hovered(&self) -> Self::Hover;
    fn set_hovered(&mut self, hover: Self::Hover);
}

/// Same idea as 'sync_panel_rect', but for hover state
/// this handles "did it change, and if so redraw".
pub fn sync_panel_hover<P: HoverablePanel>(
    panel: &mut P,
    new_hover: P::Hover,
    damage_rects: &mut Vec<DamageZone>,
    dirty_mask: &mut u32,
) -> bool {
    if new_hover == panel.hovered() {
        return false;
    }
    panel.set_hovered(new_hover);
    panel.set_dirty();

    let monitor_idx = panel.monitor_idx();
    if let Some(rect) = panel.rect() {
        emit_panel_damage(rect, monitor_idx, damage_rects, dirty_mask);
    }
    true
}

// ==========================================
// Generic animation tick
// ==========================================
//
// One abstract function 'tick_panel_animation' for every animated panel.
// answers to how much time passed, how many fixed steps to simulate this frame,
// and what damage rect to emit if anything moved

pub trait AnimatedPanel: UiPanel {
    fn last_tick(&self) -> Option<Instant>;
    fn set_last_tick(&mut self, at: Option<Instant>);

    fn anim_interval(&self) -> Duration {
        anim::frame()
    }
    fn anim_dt(&self) -> f32 {
        anim::dt()
    }

    /// Advance the animation by exactly one fixed step of 'dt' seconds.
    /// Return `true` if any visible state changed this step.
    /// Default: nothing to animate.
    fn animate_step(&mut self, _dt: f32) -> bool {
        false
    }

    fn is_animating(&self) -> bool {
        false
    }
}

/// A frame that arrived late is simulated in several steps, but only so many:
/// after a long stall the animation jumps rather than replaying every step.
const MAX_CATCH_UP_STEPS: u32 = 4;

pub fn tick_panel_animation<P: AnimatedPanel>(
    panel: &mut P,
    damage_rects: &mut Vec<DamageZone>,
    dirty_mask: &mut u32,
) {
    let now = Instant::now();
    let interval = panel.anim_interval();
    let dt = panel.anim_dt();

    let elapsed = panel
        .last_tick()
        .map(|t| now.duration_since(t))
        .unwrap_or(interval);

    let mut left = elapsed.as_secs_f32().min(dt * MAX_CATCH_UP_STEPS as f32);

    let old_render_pos = panel.render_pos();
    let mut changed = false;

    while left > 0.0 {
        let step = left.min(dt);
        if panel.animate_step(step) {
            changed = true;
        }
        left -= step;
    }
    panel.set_last_tick(panel.is_animating().then_some(now));

    if !changed {
        return;
    }

    panel.set_dirty();
    let monitor_idx = panel.monitor_idx();
    mark_dirty(dirty_mask, monitor_idx);

    let (w, h) = panel.size();
    let old_rect = Rect::from_xywh(old_render_pos.0, old_render_pos.1, w, h);
    let new_rect = panel.rect();

    let union = match (old_rect, new_rect) {
        (Some(a), Some(b)) => Rect::from_ltrb(
            a.left().min(b.left()),
            a.top().min(b.top()),
            a.right().max(b.right()),
            a.bottom().max(b.bottom()),
        ),
        (Some(a), None) | (None, Some(a)) => Some(a),
        (None, None) => None,
    };

    if let Some(rect) = union {
        damage_rects.push(DamageZone::Local { monitor_idx, rect });
    }
}
