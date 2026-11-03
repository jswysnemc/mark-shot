use tiny_skia::Rect;

use crate::renderer::Painter;
use crate::renderer::text::TextStyle;
use crate::theme::{font, radius};
use crate::ui::toast::Toasts;
/// clip is area of the screen cleared for current frame
/// draw somewhat transparent toast only if it intersects with clip, otherwise
/// the background will be drawn oveer the old frame a second time and darken it
pub fn draw_toasts(p: &mut Painter, toasts: &Toasts, monitor_idx: usize, clip: Option<&Rect>) {
    let cfg = &crate::config::get().toasts;
    for toast in &toasts.items {
        if toast.monitor_idx != monitor_idx {
            continue;
        }
        let Some(rect) = toast.rect else { continue };

        let opacity = toast.opacity();
        if opacity <= 0.0 {
            continue;
        }
        if let Some(clip) = clip
            && (clip.left() >= rect.right()
                || clip.right() <= rect.left()
                || clip.top() >= rect.bottom()
                || clip.bottom() <= rect.top())
        {
            continue;
        }

        p.fill_rounded(rect, radius::panel(), cfg.background.get().fade(opacity));

        p.panel_border(
            rect.left(),
            rect.top(),
            rect.width(),
            rect.height(),
            radius::panel(),
            opacity,
        );

        p.text(
            rect,
            toast.text.as_ref(),
            TextStyle::new(font::label(), cfg.text.get().fade(opacity)).center(),
        );
    }
}
