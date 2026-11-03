// Input behavior logic (thresholds are in config [input]): double-clicks, scroll accumulation, hit-test and etc
use std::time::{Duration, Instant};

use crate::ui::color_popover::ColorField;

// ==========================================
// Double click
// ==========================================

/// A double-click is counted only if all of:
/// 1. Both clicks occur on the exact same target
/// 2. Time elapsed between the two clicks is less than `input.double_click_ms`.
/// 3. Distance between the two clicks is within `input.double_click_distance`.
#[derive(Debug, Default)]
pub struct DoubleClickTracker<T> {
    last: Option<(Instant, T, (f32, f32))>,
}

impl<T: PartialEq + Copy> DoubleClickTracker<T> {
    pub fn new() -> Self {
        Self { last: None }
    }

    /// 【鼠标输入】【点击重置】清除旧点击记录，拖动或其他按钮操作不能组成双击
    pub fn clear(&mut self) {
        self.last = None;
    }

    /// Registers a single click on a `target` at a specific position `pos`.
    ///
    /// The `pos` coordinates can be either local or global. The tracker only
    /// calculates relative distance, so the coordinate system doesn't matter
    /// as long as the caller is consistent.
    ///
    /// # Returns
    /// * `true` if this click completes a valid double-click sequence.
    /// * `false` if it is the first click, took too long, moved too far,
    ///   or was on a different target.
    pub fn register(&mut self, target: T, pos: (f32, f32)) -> bool {
        let now = Instant::now();
        let input = &crate::config::get().input;
        let is_double = self.last.is_some_and(|(t, prev_target, prev_pos)| {
            prev_target == target
                && now.duration_since(t) < Duration::from_millis(input.double_click_ms)
                && dist(prev_pos, pos) <= input.double_click_distance
        });

        self.last = Some((now, target, pos));

        is_double
    }
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

// ==========================================
// Straight lines
// ==========================================

pub fn snap_line(start: (f32, f32), end: (f32, f32)) -> (f32, f32) {
    let max = crate::config::get().input.snap_angle.to_radians().tan();
    let (dx, dy) = ((end.0 - start.0).abs(), (end.1 - start.1).abs());
    if dy <= dx * max {
        (end.0, start.1)
    } else if dx <= dy * max {
        (start.0, end.1)
    } else {
        end
    }
}

/// Identifiers for interactive objects that support double-click tracking.
///
/// This serves as the target type `T` for the `DoubleClickTracker`, allowing
/// the system to distinguish between clicks on different UI elements or annotations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickTarget {
    SettingsWidget(usize),
    TextAnnotation(u64),
    ColorField(ColorField),
    OcrLine(usize),
    Selection,
}

// ==========================================
// Scroll accumulator
// ==========================================
//
// Shared rate-limited scroll accumulator used by panels that support
// scroll-to-step behaviour (exp: color fields, stepper widgets).
// (its implemented for the first place when scrolling using trackpad or holding mouse wheel,
// since that floods thousands of events that would freeze the app)
// 1. **Rate limit** : any event arriving sooner than `MIN_INTERVAL` after the
//    last *processed* event is dropped entirely, without accumulation.
//    This is nearly free (just one `Instant` comparison) and prevents every
//    raw event from triggering a full apply + rebuild + damage cycle.
//
// 2. **Step cap** : even if many fractional steps accumulated, at most
//    `MAX_STEPS_PER_CALL` whole steps are returned per call. The remainder
//    is discarded rather than carried forward to avoid a "debt" that would
//    keep firing long after the user stopped scrolling.

/// Rate-limited scroll accumulator.
///
/// `K` is the "slot" key, aka the widget or field the scroll targets.
/// Switching to a different key resets the leftover accumulator.
pub struct ScrollAccumulator<K: PartialEq> {
    /// The key whose scroll is currently being accumulated.
    pub current_key: Option<K>,
    /// Fractional steps not yet converted to whole steps.
    pub accumulator: f32,
    /// When the last event was actually processed (not rate-limited away).
    pub last_processed: Option<Instant>,
}

impl<K: PartialEq> Default for ScrollAccumulator<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: PartialEq> ScrollAccumulator<K> {
    pub fn new() -> Self {
        Self {
            current_key: None,
            accumulator: 0.0,
            last_processed: None,
        }
    }

    /// Feed a fractional `delta` (in steps, not raw pixels) for the given
    /// `key` and return the number of whole steps to apply (may be negative).
    pub fn step(&mut self, key: K, delta: f32) -> i32 {
        const MIN_INTERVAL: Duration = Duration::from_millis(12);
        const MAX_STEPS_PER_CALL: i32 = 3;

        // Switching target: discard leftover from the previous slot.
        if self.current_key.as_ref() != Some(&key) {
            self.current_key = Some(key);
            self.accumulator = 0.0;
            self.last_processed = None;
        }

        // Direction reversal: don't let the leftover from the previous
        // direction "absorb" the new one — reset and start fresh.
        if delta != 0.0 && self.accumulator != 0.0 && self.accumulator.signum() != delta.signum() {
            self.accumulator = 0.0;
        }

        // Rate limit: drop events that arrive too quickly.
        if let Some(last) = self.last_processed
            && Instant::now().duration_since(last) < MIN_INTERVAL
        {
            return 0;
        }
        self.last_processed = Some(Instant::now());

        self.accumulator += delta;

        let mut steps = 0i32;
        while self.accumulator >= 1.0 && steps < MAX_STEPS_PER_CALL {
            steps += 1;
            self.accumulator -= 1.0;
        }
        while self.accumulator <= -1.0 && steps > -MAX_STEPS_PER_CALL {
            steps -= 1;
            self.accumulator += 1.0;
        }
        // Remaining accumulator still exceeds a full step: discard it
        // rather than carrying rest into future calls.
        if self.accumulator.abs() > 1.0 {
            self.accumulator = 0.0;
        }
        steps
    }

    /// scroll-inertia reset. Call on any non-scroll user action
    /// (click, key press) so stale queued events don't fire afterwards.
    pub fn cancel(&mut self) {
        self.current_key = None;
        self.accumulator = 0.0;
        self.last_processed = None;
    }
}
