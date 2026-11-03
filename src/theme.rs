// color, radius, stroke, font, anim, shadow and etc, everything visual.
// but here are only tokens that are shared between at least two components.
// if not, its constants stay in their prespective file.

use tiny_skia::Color;

// ==========================================
// Color
// ==========================================

/// rgba struct since there is multiple definition of color
/// (tiny_skia::color, usvg::color, (u8,u8,u8,u8) tuple in ocr)
/// while we want to have a single source of truth
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    pub fn color(self) -> Color {
        Color::from_rgba8(self.0, self.1, self.2, self.3)
    }

    /// for transparency animations multiplying the alpha of the token by `k`
    pub fn fade(self, k: f32) -> Color {
        let mut c = self.color();
        c.set_alpha(c.alpha() * k.clamp(0.0, 1.0));
        c
    }

    pub fn cosmic(self) -> cosmic_text::Color {
        cosmic_text::Color::rgba(self.0, self.1, self.2, self.3)
    }

    pub fn usvg(self) -> usvg::Color {
        usvg::Color {
            red: self.0,
            green: self.1,
            blue: self.2,
        }
    }

    pub fn alpha(self) -> u8 {
        self.3
    }

    pub fn with_alpha(self, alpha: u8) -> Self {
        Self(self.0, self.1, self.2, alpha)
    }

    /// Parses "#RRGGBB" or "#RRGGBBAA" (case-insensitive, alpha defaults to 255).
    pub fn parse_hex(s: &str) -> Option<Self> {
        let s = s.strip_prefix('#').unwrap_or(s);
        let byte = |i: usize| u8::from_str_radix(s.get(i..i + 2)?, 16).ok();
        match s.len() {
            6 => Some(Self(byte(0)?, byte(2)?, byte(4)?, 255)),
            8 => Some(Self(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => None,
        }
    }
}

impl From<Rgba> for Color {
    fn from(v: Rgba) -> Self {
        v.color()
    }
}

pub mod color {
    use super::Rgba;

    fn theme() -> &'static crate::config::Theme {
        &crate::config::get().theme
    }

    /// background color of all panels, popovers and toasts
    pub fn panel() -> Rgba {
        theme().background
    }
    /// main color of elements on the panel, like text, seperators and etc
    pub fn foreground() -> Rgba {
        theme().foreground
    }
    /// secondary small labels
    pub fn muted() -> Rgba {
        theme().muted
    }
    /// hovering color
    pub fn hover() -> Rgba {
        theme().hover
    }
    /// selected color (`theme.accent`)
    pub fn active() -> Rgba {
        theme().accent
    }
    /// icons and text over a hover or active fill
    pub fn on_active() -> Rgba {
        theme().on_accent
    }
    /// blue color of text selection
    pub fn text_selection() -> Rgba {
        theme().text_selection
    }
    /// input field
    pub fn field() -> Rgba {
        theme().field
    }
    /// empty part of downloading bar
    pub fn track() -> Rgba {
        theme().track
    }
    pub fn caret() -> Rgba {
        theme().caret
    }
    /// panels border
    pub fn border() -> Rgba {
        theme().border
    }
    /// panels border on a light background
    pub fn border_on_light() -> Rgba {
        theme().border_on_light
    }
}

// ==========================================
// Panel geometry
// ==========================================

pub mod size {
    /// Height of the toolbar and of the settings panel.
    pub fn panel_height() -> f32 {
        crate::config::get().theme.panel_height
    }
    /// Padding between a panel's edge and its items.
    pub fn padding() -> f32 {
        crate::config::get().theme.panel_padding
    }
    /// Gap between a panel and whatever it is anchored to.
    pub fn margin() -> f32 {
        crate::config::get().theme.panel_margin
    }
}

// ==========================================
// Border Radius
// ==========================================

pub mod radius {
    pub fn panel() -> f32 {
        crate::config::get().theme.panel_radius
    }
    pub fn item() -> f32 {
        crate::config::get().theme.item_radius
    }
    pub fn separator() -> f32 {
        crate::config::get().theme.separator_radius
    }
}

// ==========================================
// Stroke width
// ==========================================

pub mod stroke {
    pub fn border() -> f32 {
        crate::config::get().theme.border_width
    }
    /// downloading bar stroke
    pub fn progress() -> f32 {
        crate::config::get().theme.progress_height
    }
}

// ==========================================
// font
// ==========================================

pub mod font {
    pub fn label() -> f32 {
        crate::config::get().theme.font_size
    }
    pub fn small() -> f32 {
        crate::config::get().theme.small_font_size
    }
    /// Line height of the UI text as a factor of the font size.
    pub fn line_height() -> f32 {
        crate::config::get().theme.line_height
    }
}

// ==========================================
// Animation
// ==========================================

pub mod anim {
    use std::time::Duration;

    /// The duration of a single frame in milliseconds.
    pub fn frame() -> Duration {
        Duration::from_millis(crate::config::get().animation.frame_ms)
    }
    /// A frame in seconds.
    pub fn dt() -> f32 {
        crate::config::get().animation.frame_ms as f32 / 1000.0
    }
    /// Opacity per second while a popover fades in or out.
    pub fn popover_fade() -> f32 {
        crate::config::get().animation.popover_fade
    }
    /// Closer than this to the target opacity counts as settled.
    pub const OPACITY_EPSILON: f32 = 0.001;
}

// ==========================================
// Annotations's shadow
// ==========================================

pub mod shadow {
    /// How far the shadow reaches past a stroke, for the damaged zone calculation.
    pub fn width_bonus() -> f32 {
        let shadow = &crate::config::get().annotations.shadow;
        shadow.layers as f32 * shadow.spread + 1.0
    }
}
