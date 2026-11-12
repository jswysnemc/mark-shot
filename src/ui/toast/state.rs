// notifications: simple text on the overlay, without any input or hit testing
// not implemented as panels, since panels are not a one time thing, and has wider
// functionality, as input fields, hit testing and etc
use std::borrow::Cow;
use std::time::{Duration, Instant};

use cosmic_text::FontSystem;
use tiny_skia::Rect;

use crate::editor::DamageZone;
use crate::theme::anim;
use crate::ui::panel::emit_panel_damage;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    OcrPickRegion,
    OcrNoModel,
    OcrDownloadFailed,
    OcrFailed,
    ScanFailed,
    OcrNoText,
    CopyFailed,
    ColorCopied,
    PickColor,
    ScrollNeedsRegion,
    RulerMode,
    NumberExhausted,
    HistoryFailed,
    SettingsFailed,
    SaveAsFailed,
    ExternalActionFailed,
    UploadFeedback,
    TranslationFeedback,
    SelectionTooSmall,
}

#[derive(Clone, Copy)]
pub enum ToastAnchor {
    CursorMonitorCenter,
    /// if there is selection, center of local selection in the monitor, else just center
    SelectionCenter,
}

#[derive(Clone, Copy)]
pub enum ToastLife {
    Manual,
    Timed(Duration),
}

pub struct ToastSpec {
    pub text: &'static str,
    pub anchor: ToastAnchor,
    pub life: ToastLife,
    /// in seconds
    pub fade_in: f32,
    pub fade_out: f32,
}

impl ToastKind {
    pub fn spec(self) -> ToastSpec {
        let cfg = &crate::config::get().toasts;
        let animation = &crate::config::get().animation;
        let (fade_in, fade_out) = (animation.toast_fade_in, animation.toast_fade_out);
        let timed = |secs: f32| {
            ToastLife::Timed(Duration::try_from_secs_f32(secs.max(0.0)).unwrap_or(Duration::MAX))
        };
        match self {
            ToastKind::ScanFailed => ToastSpec {
                text: "Cannot open code scanner",
                anchor: ToastAnchor::CursorMonitorCenter,
                life: timed(cfg.lifetime),
                fade_in,
                fade_out,
            },
            ToastKind::SettingsFailed => ToastSpec {
                text: "Failed to save settings",
                anchor: ToastAnchor::CursorMonitorCenter,
                life: ToastLife::Timed(Duration::from_millis(2500)),
                fade_in,
                fade_out,
            },
            ToastKind::SelectionTooSmall => ToastSpec {
                text: "Selection is too small",
                anchor: ToastAnchor::CursorMonitorCenter,
                life: ToastLife::Timed(Duration::from_millis(1400)),
                fade_in,
                fade_out,
            },
            ToastKind::SaveAsFailed
            | ToastKind::ExternalActionFailed
            | ToastKind::UploadFeedback
            | ToastKind::TranslationFeedback => ToastSpec {
                text: "Cannot save screenshot",
                anchor: ToastAnchor::CursorMonitorCenter,
                life: timed(cfg.lifetime),
                fade_in,
                fade_out,
            },
            ToastKind::HistoryFailed => ToastSpec {
                text: "Cannot open screenshot history",
                anchor: ToastAnchor::CursorMonitorCenter,
                life: timed(cfg.lifetime),
                fade_in,
                fade_out,
            },
            ToastKind::NumberExhausted => ToastSpec {
                text: "Number sequence is full; reset it to continue",
                anchor: ToastAnchor::CursorMonitorCenter,
                life: timed(cfg.lifetime),
                fade_in,
                fade_out,
            },
            ToastKind::RulerMode => ToastSpec {
                text: "Measure length",
                anchor: ToastAnchor::CursorMonitorCenter,
                life: ToastLife::Timed(Duration::from_millis(1200)),
                fade_in,
                fade_out,
            },
            ToastKind::OcrPickRegion => ToastSpec {
                text: "Drag a box over the area you want to read",
                anchor: ToastAnchor::CursorMonitorCenter,
                life: ToastLife::Manual,
                fade_in,
                fade_out,
            },
            ToastKind::OcrNoModel => ToastSpec {
                text: "Choose an OCR engine and install required resources",
                anchor: ToastAnchor::SelectionCenter,
                life: timed(cfg.lifetime),
                fade_in,
                fade_out,
            },
            ToastKind::PickColor => ToastSpec {
                text: "Click anywhere to pick a color",
                anchor: ToastAnchor::SelectionCenter,
                life: ToastLife::Manual,
                fade_in,
                fade_out,
            },
            ToastKind::ColorCopied => ToastSpec {
                text: "Copied",
                anchor: ToastAnchor::SelectionCenter,
                life: timed(cfg.copied_lifetime),
                fade_in,
                fade_out,
            },
            ToastKind::OcrDownloadFailed => ToastSpec {
                text: "Couldn't download the language, check the connection",
                anchor: ToastAnchor::SelectionCenter,
                life: timed(cfg.lifetime),
                fade_in,
                fade_out,
            },
            ToastKind::OcrFailed => ToastSpec {
                text: "Couldn't read this area",
                anchor: ToastAnchor::SelectionCenter,
                life: timed(cfg.lifetime),
                fade_in,
                fade_out,
            },
            ToastKind::OcrNoText => ToastSpec {
                text: "No text found here",
                anchor: ToastAnchor::SelectionCenter,
                life: timed(cfg.short_lifetime),
                fade_in,
                fade_out,
            },
            ToastKind::CopyFailed => ToastSpec {
                text: "Couldn't copy to the clipboard",
                anchor: ToastAnchor::SelectionCenter,
                life: timed(cfg.lifetime),
                fade_in,
                fade_out,
            },
            ToastKind::ScrollNeedsRegion => ToastSpec {
                text: "Select the area to scroll-capture first",
                anchor: ToastAnchor::CursorMonitorCenter,
                life: timed(cfg.lifetime),
                fade_in,
                fade_out,
            },
        }
    }
}

pub struct Toast {
    pub kind: ToastKind,
    pub spec: ToastSpec,
    pub text: Cow<'static, str>,
    pub text_width: f32,
    pub monitor_idx: usize,
    pub rect: Option<Rect>,
    pub progress: f32,
    pub closing: bool,
    pub shown_at: Instant,
}

impl Toast {
    pub fn opacity(&self) -> f32 {
        let t = self.progress.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    fn layout(&mut self, place: &ToastPlace) {
        self.monitor_idx = place.monitor_idx;
        let cfg = &crate::config::get().toasts;
        let (height, margin) = (cfg.height, cfg.margin);
        let w = self.text_width + cfg.padding * 2.0;
        let (cx, cy) = match (self.spec.anchor, place.focus) {
            (ToastAnchor::SelectionCenter, Some(focus)) => (
                focus.left() + focus.width() / 2.0,
                focus.top() + focus.height() / 2.0,
            ),
            _ => (place.size.0 / 2.0, place.size.1 / 2.0),
        };
        let x = (cx - w / 2.0)
            .max(margin)
            .min((place.size.0 - w - margin).max(margin));
        let y = (cy - height / 2.0)
            .max(margin)
            .min((place.size.1 - height - margin).max(margin));
        self.rect = Rect::from_xywh(x.round(), y.round(), w, height);
    }
}

#[derive(Clone, Copy)]
pub struct ToastPlace {
    pub monitor_idx: usize,
    pub size: (f32, f32),
    pub focus: Option<Rect>,
}

/// Longest step a single tick may advance by, so a stalled frame doesn't
/// fast-forward a toast through its whole fade.
const MAX_STEP: f32 = 0.1;

#[derive(Default)]
pub struct Toasts {
    pub items: Vec<Toast>,
    last_tick: Option<Instant>,
}

impl Toasts {
    /// Shows a toast with predefined static text.
    pub fn show(&mut self, kind: ToastKind, font_system: &mut FontSystem) {
        self.push(kind, Cow::Borrowed(kind.spec().text), font_system);
    }

    /// Shows a toast with dynamic text evaluated at runtime.
    pub fn show_text(&mut self, kind: ToastKind, text: String, font_system: &mut FontSystem) {
        self.push(kind, Cow::Owned(text), font_system);
    }

    fn push(&mut self, kind: ToastKind, text: Cow<'static, str>, font_system: &mut FontSystem) {
        let text_width =
            crate::renderer::measure_line_width(&text, crate::theme::font::label(), font_system);
        if let Some(toast) = self.items.iter_mut().find(|t| t.kind == kind) {
            toast.closing = false;
            toast.shown_at = Instant::now();
            toast.text = text;
            toast.text_width = text_width;
            return;
        }

        let spec = kind.spec();
        self.items.push(Toast {
            kind,
            spec,
            text,
            text_width,
            monitor_idx: 0,
            rect: None,
            progress: 0.0,
            closing: false,
            shown_at: Instant::now(),
        });
    }

    pub fn dismiss(&mut self, kind: ToastKind) {
        for toast in self.items.iter_mut().filter(|t| t.kind == kind) {
            toast.closing = true;
        }
    }

    /// 【界面提示】【动画资格】返回是否仍需淡入或淡出帧，无参数；完整显示的限时提示只等待到期时间
    pub fn is_animating(&self) -> bool {
        self.items.iter().any(|t| t.closing || t.progress < 1.0)
    }

    /// 【界面提示】【到期期限】返回尚未关闭的限时提示最早期限，无参数，不为稳定画面安排周期帧
    pub fn deadline(&self) -> Option<Instant> {
        self.items
            .iter()
            .filter(|toast| !toast.closing)
            .filter_map(|toast| match toast.spec.life {
                ToastLife::Timed(duration) => toast.shown_at.checked_add(duration),
                ToastLife::Manual => None,
            })
            .min()
    }

    pub fn tick(
        &mut self,
        place: ToastPlace,
        damage_rects: &mut Vec<DamageZone>,
        dirty_mask: &mut u32,
    ) {
        if self.items.is_empty() {
            return;
        }

        let now = Instant::now();
        let elapsed = self
            .last_tick
            .map(|t| now.duration_since(t))
            .unwrap_or(anim::frame());
        self.last_tick = Some(now);
        let dt = elapsed.as_secs_f32().min(MAX_STEP);
        let speed = crate::config::get().animation.speed;

        self.items.retain_mut(|toast| {
            let was = (toast.monitor_idx, toast.rect);

            if !toast.closing
                && let ToastLife::Timed(after) = toast.spec.life
                && toast.shown_at.elapsed() >= after
            {
                toast.closing = true;
            }

            let rate = if toast.closing {
                -1.0 / toast.spec.fade_out
            } else {
                1.0 / toast.spec.fade_in
            };
            let next = (toast.progress + rate * speed * dt).clamp(0.0, 1.0);
            let faded = next != toast.progress;
            toast.progress = next;

            toast.layout(&place);

            if faded || was != (toast.monitor_idx, toast.rect) {
                if let Some(rect) = was.1 {
                    emit_panel_damage(rect, was.0, damage_rects, dirty_mask);
                }
                if let Some(rect) = toast.rect {
                    emit_panel_damage(rect, toast.monitor_idx, damage_rects, dirty_mask);
                }
            }

            !(toast.closing && toast.progress <= 0.0)
        });

        if self.items.is_empty() {
            self.last_tick = None;
        }
    }
}
