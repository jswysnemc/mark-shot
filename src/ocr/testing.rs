// Common OCR test utilities
// mock engine instances, sample images, and temporary directory helpers.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use tiny_skia::Rect;

use super::models::ModelFiles;
use super::{CANCELLED, OcrBackend, OcrError, OcrImage, OcrLine, OcrText};

#[derive(Clone)]
pub struct Fake {
    pub label: String,
    pub steps: usize,
    pub step: Duration,
    pub fail: Option<String>,
    pub panic: bool,
    /// deaf in terms can't heard canceled signal
    pub deaf: bool,
    pub calls: Arc<AtomicUsize>,
    pub cancels: Arc<AtomicUsize>,
}

impl Fake {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_owned(),
            steps: 0,
            step: Duration::ZERO,
            fail: None,
            panic: false,
            deaf: false,
            calls: Arc::default(),
            cancels: Arc::default(),
        }
    }

    pub fn slow(mut self, steps: usize, step: Duration) -> Self {
        self.steps = steps;
        self.step = step;
        self
    }

    pub fn failing(mut self, message: &str) -> Self {
        self.fail = Some(message.to_owned());
        self
    }

    pub fn panicking(mut self) -> Self {
        self.panic = true;
        self
    }

    pub fn deaf(mut self) -> Self {
        self.deaf = true;
        self
    }

    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    pub fn cancels(&self) -> usize {
        self.cancels.load(Ordering::SeqCst)
    }
}

impl OcrBackend for Fake {
    fn recognize(
        &self,
        image: OcrImage,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<OcrText, OcrError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.panic {
            panic!("fake backend panicked on purpose");
        }
        for _ in 0..self.steps {
            if !self.deaf && cancelled() {
                self.cancels.fetch_add(1, Ordering::SeqCst);
                return Err(CANCELLED.into());
            }
            std::thread::sleep(self.step);
        }
        if let Some(message) = &self.fail {
            return Err(message.clone().into());
        }
        Ok(labelled(&self.label, &image))
    }
}

/// one line: who read the image and what is its size
pub fn labelled(label: &str, image: &OcrImage) -> OcrText {
    let text = format!("{label} {}x{}", image.width, image.height);
    let n = text.chars().count();
    let (x, y) = image.origin;
    let bounds = Rect::from_xywh(x, y, image.width as f32, image.height as f32).unwrap();
    let char_x = (0..=n)
        .map(|k| x + image.width as f32 * k as f32 / n as f32)
        .collect();
    OcrText {
        lines: vec![OcrLine {
            text,
            bounds,
            char_x,
        }],
    }
}

pub fn label_of(text: &OcrText) -> String {
    text.lines[0].text.split(' ').next().unwrap().to_owned()
}

pub fn image(width: u32, height: u32) -> OcrImage {
    OcrImage {
        rgb: (0..width * height * 3).map(|i| (i % 251) as u8).collect(),
        width,
        height,
        origin: (10.0, 20.0),
    }
}

pub fn files(model: &str) -> ModelFiles {
    ModelFiles {
        detector: PathBuf::from("/models/det.onnx"),
        recognizer: PathBuf::from(format!("/models/{model}.onnx")),
        dict: PathBuf::from(format!("/models/{model}.txt")),
    }
}

// Temporary test directory; automatically cleaned up when dropped.
pub struct TempDir(PathBuf);

impl std::ops::Deref for TempDir {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn temp_dir(tag: &str) -> TempDir {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let base = std::env::temp_dir();
    let base = if base.as_os_str().len() > 48 {
        PathBuf::from("/tmp")
    } else {
        base
    };
    let dir = base.join(format!(
        "lumine-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    TempDir(dir)
}

pub fn wait_until(timeout: Duration, mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if done() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
