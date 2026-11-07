// Optical character recognition.
//
// `OcrBackend` hides the engine: input `OcrImage`, output `OcrText`.
// implementation of new engine is a new module & one line in `default_backend`.
//
// layout   - detection boxes -> lines, blocks, reading order
// view     - selection over those lines
// runtime  - recognition on a worker thread
// models   - available, installed and selected models
// download - background download of models
// bidi     - converting visual RTL text order to logical order for copy/paste.
// daemon   - persistent process keeping a pre-ready model instance ready in memory. (crucial for gpu ocr, see below)
// settings - how to prepare the engine (on-demand / at-launch / daemon)

// clarifications:
// Settings :
// - on-demand : load the engine when choosing ocr
// - at-launch : load the engine at app launch
// - daemon    : launches a background daemon, that outlives main process, which
//               purpose is keeping ready ocr engine

// When specific mode is chosen and why:
// first things to keep in mind:
// * GPU initialization takes time, and decent chunk on memory (`1`)
// * Preparing engine for cpu takes around 90ms, and some memory (`2`)

// Default mode: on-demand.
// 1. why not at-launch: launching engine on every launch is inefficient, it's not something user will use on every launch
//  it will make sense to make a separate bind, that instantly launches the engines, and chooses ocr tool, but that's later

// 2. why not daemon: launching daemon for cpu is inefficient, running a separate daemon will gain practically nothing, only around
// 90 ms on my machine. Yet, for gpu there is real gain, gpu scan is much faster than cpu, and launching engine with gpu
// takes much more time, so it makes sense for it to be in a daemon. Yet, daemon takes GPU memory, and having a background
// daemon with always 100+mb taken is not a good idea, sooo it's not optimal
pub mod bidi;
#[cfg(target_os = "linux")]
pub mod daemon;
#[cfg(target_os = "windows")]
#[path = "ocr/daemon_windows.rs"]
pub mod daemon;
pub mod dawn;
pub mod download;
pub mod draw;
pub mod layout;
pub mod lookalikes;
pub mod model_integrity;
pub mod models;
pub mod paddle_backend;
mod parallelism;
pub mod preprocess;
pub mod runtime;
pub mod settings;
#[cfg(test)]
pub mod testing;
pub mod view;
pub mod worker;

use tiny_skia::Rect;

pub use runtime::{OcrRuntime, StartOutcome};
pub use view::OcrView;

use crate::types::Placement;
use models::ModelFiles;

/// Tightly packed RGB8 pixels plus the global coordinate of their top-left
/// corner, so results can be mapped back onto the canvas. Owned: it moves to
/// the worker thread and into the backend without another copy.
pub struct OcrImage {
    pub rgb: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub origin: (f32, f32),
}

/// One recognized line. `bounds` is global. `char_x` is the global x of every
/// character boundary: `text.chars().count() + 1` values, non-decreasing, and
/// inside `bounds`, the first and last on its edges.
#[derive(Debug, Clone, PartialEq)]
pub struct OcrLine {
    pub text: String,
    pub bounds: Rect,
    pub char_x: Vec<f32>,
}

impl OcrLine {
    pub fn char_count(&self) -> usize {
        self.char_x.len().saturating_sub(1)
    }
}

/// Checks if a character is a combining mark (diacritical mark that attaches
/// to the previous letter without taking up horizontal spacing).
pub fn is_mark(c: char) -> bool {
    matches!(
        c as u32,
        0x0300..=0x036F
            | 0x0483..=0x0489
            | 0x0591..=0x05C7
            | 0x0610..=0x061A
            | 0x064B..=0x065F
            | 0x0670
            | 0x06D6..=0x06ED
            | 0x0E31
            | 0x0E34..=0x0E3A
            | 0x0E47..=0x0E4E
            | 0x1AB0..=0x1AFF
            | 0x20D0..=0x20FF
            | 0x3099..=0x309A
    )
}

/// Everything found in one image, lines in reading order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OcrText {
    pub lines: Vec<OcrLine>,
}

impl OcrText {
    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(|l| l.text.trim().is_empty())
    }
}

pub type OcrError = Box<dyn std::error::Error + Send + Sync>;

/// cancelled text scan
pub const CANCELLED: &str = "cancelled";

/// `Send` so recognition can run on a worker thread. Takes the image by value
/// to avoid copying the full screen buffer.
/// Checked between pipeline stages so cancelled scans exit early with an error.
pub trait OcrBackend: Send {
    fn recognize(&self, image: OcrImage, cancelled: &dyn Fn() -> bool)
    -> Result<OcrText, OcrError>;
}

/// Build the backend the app ships with today.
pub fn default_backend(files: &ModelFiles) -> Result<Box<dyn OcrBackend>, OcrError> {
    Ok(Box::new(paddle_backend::PaddleBackend::new(files)?))
}

/// Connects to the daemon client if running in daemon mode
/// otherwise, initializes the engine directly within this process
pub fn build_backend(
    mode: settings::Mode,
    files: &ModelFiles,
) -> Result<Box<dyn OcrBackend>, OcrError> {
    match mode {
        settings::Mode::Daemon => daemon::client::connect(files),
        settings::Mode::OnDemand | settings::Mode::AtLaunch => default_backend(files),
    }
}

/// Composite the pixels covered by `region` (global coords) out of the
/// per-monitor captures into one contiguous RGB8 buffer, without annotations.
pub fn composite_region(
    captures: &[crate::types::Capture],
    placements: &[Placement],
    region: Rect,
) -> Option<OcrImage> {
    let (out, (left, top), _) =
        crate::renderer::composite(captures, placements, region, Some(1.0))?;
    let (width, height) = (out.width(), out.height());

    // RGBA -> RGB
    let rgba = out.data();
    let mut rgb = vec![0u8; (width as usize) * (height as usize) * 3];
    for (dst, src) in rgb
        .as_chunks_mut::<3>()
        .0
        .iter_mut()
        .zip(rgba.as_chunks::<4>().0)
    {
        dst.copy_from_slice(&src[..3]);
    }

    Some(OcrImage {
        rgb,
        width,
        height,
        origin: (left as f32, top as f32),
    })
}
