use log::info;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use crate::ocr::models::ModelFiles;
use crate::ocr::paddle_backend::PaddleBackend;
use crate::ocr::settings::GPU_BUILD;
use crate::ocr::{OcrImage, OcrText};

use super::unix_now;

pub const FORMAT: u32 = 1;

/// margin between gpu and cpu, how much gpu needs to win to bo chosen in the auto mode
const MARGIN: f64 = 0.8;

const PROBE_WIDTH: u32 = 1280;
const PROBE_HEIGHT: u32 = 720;
const PROBE_LINES: u32 = 12;
const LINE_HEIGHT: u32 = 56;
const SCANS: usize = 2;

/// stuck GPU driver must not keep the daemon in `Loading` forever.
pub const BUDGET: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Gpu,
    Cpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    pub verdict: Verdict,
    pub cpu_ms: Option<u32>,
    pub gpu_ms: Option<u32>,
    pub at: u64,
}

impl Record {
    pub fn speedup(&self) -> Option<f64> {
        let (cpu, gpu) = (self.cpu_ms?, self.gpu_ms?);
        (gpu > 0).then(|| f64::from(cpu) / f64::from(gpu))
    }

    pub fn summary(&self) -> String {
        let side = |ms: Option<u32>| ms.map_or("failed".to_owned(), |ms| format!("{ms} ms"));
        let times = format!("CPU {}, GPU {}", side(self.cpu_ms), side(self.gpu_ms));
        match (self.verdict, self.speedup()) {
            (Verdict::Gpu, Some(times_faster)) => {
                format!("GPU is {times_faster:.1}x faster ({times})")
            }
            (Verdict::Gpu, None) => format!("GPU wins ({times})"),
            (Verdict::Cpu, Some(times_faster)) if times_faster > 0.0 => {
                format!("CPU is {:.1}x faster ({times})", 1.0 / times_faster)
            }
            (Verdict::Cpu, _) => format!("CPU wins ({times})"),
        }
    }
}

pub fn decide(cpu_ms: Option<u32>, gpu_ms: Option<u32>) -> Verdict {
    match (cpu_ms, gpu_ms) {
        (Some(cpu), Some(gpu)) if f64::from(gpu) <= f64::from(cpu) * MARGIN => Verdict::Gpu,
        (None, Some(_)) => Verdict::Gpu,
        _ => Verdict::Cpu,
    }
}

pub fn cache_path() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join("mark-shot").join("ocr-device"))
}

/// Identifies the machine the benchmarks were taken on: installed GPUs,
/// driver version, and engine build (triggers re-benchmarking if GPU or driver changes)
pub fn machine_key() -> String {
    let mut parts = vec![format!("v{FORMAT}"), env!("CARGO_PKG_VERSION").to_owned()];
    parts.extend(graphics_cards());
    if let Ok(version) = fs::read_to_string("/sys/module/nvidia/version") {
        parts.push(format!("nvidia-{}", version.trim()));
    }
    parts.join(" ")
}

fn graphics_cards() -> Vec<String> {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return Vec::new();
    };
    let mut cards: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("render"))
        .filter_map(|entry| {
            let device = entry.path().join("device");
            let id = |file: &str| {
                fs::read_to_string(device.join(file))
                    .ok()
                    .map(|value| value.trim().to_owned())
            };
            Some(format!("{}:{}", id("vendor")?, id("device")?))
        })
        .collect();
    cards.sort();
    cards.dedup();
    cards
}

pub fn stored(path: &Path, key: &str) -> Option<Record> {
    parse(&fs::read_to_string(path).ok()?, key)
}

pub fn store(path: &Path, record: &Record, key: &str) {
    let written = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::write(path, render(record, key)));
    if let Err(e) = written {
        info!("ocr: cannot save the device measurement: {e}");
    }
}

pub fn render(record: &Record, key: &str) -> String {
    let side = |ms: Option<u32>| ms.map_or("failed".to_owned(), |ms| ms.to_string());
    let verdict = match record.verdict {
        Verdict::Gpu => "gpu",
        Verdict::Cpu => "cpu",
    };
    format!(
        "format = {FORMAT}\nkey = {key}\nverdict = {verdict}\ncpu_ms = {}\ngpu_ms = {}\nat = {}\n",
        side(record.cpu_ms),
        side(record.gpu_ms),
        record.at,
    )
}

pub fn parse(text: &str, key: &str) -> Option<Record> {
    let mut fields: Vec<(&str, &str)> = Vec::new();
    for line in text.lines() {
        if let Some((name, value)) = line.split_once('=') {
            fields.push((name.trim(), value.trim()));
        }
    }
    let field = |name: &str| {
        fields
            .iter()
            .find(|(found, _)| *found == name)
            .map(|(_, value)| *value)
    };
    if field("format")? != FORMAT.to_string() || field("key")? != key {
        return None;
    }
    let ms = |name: &str| field(name).and_then(|value| value.parse().ok());
    Some(Record {
        verdict: match field("verdict")? {
            "gpu" => Verdict::Gpu,
            "cpu" => Verdict::Cpu,
            _ => return None,
        },
        cpu_ms: ms("cpu_ms"),
        gpu_ms: ms("gpu_ms"),
        at: ms("at").map(u64::from).unwrap_or_default(),
    })
}

/// Measures the execution time of a single warm scan on each device. Runs on the daemon's
/// engine thread so the overlay process can continue recognizing text while this runs.
pub fn run(files: &ModelFiles, save: &dyn Fn(&Record)) -> Record {
    info!("ocr-daemon: measuring this machine once (CPU against GPU), takes a few seconds");
    let cpu_ms = guarded(files, false);
    let mut record = Record {
        verdict: decide(cpu_ms, None),
        cpu_ms,
        gpu_ms: None,
        at: unix_now(),
    };
    if GPU_BUILD {
        // FIX
        // If gpu fails, the whoole process will crash, record remains "CPU, GPU failed"
        // and the measurement will not be repeated in an inifnite glitched loop
        save(&record);
        record.gpu_ms = guarded(files, true);
        record.verdict = decide(cpu_ms, record.gpu_ms);
    }
    save(&record);
    info!("ocr-daemon: measured, {}", record.summary());
    record
}

fn guarded(files: &ModelFiles, gpu: bool) -> Option<u32> {
    let (tx, rx) = channel();
    let files = files.clone();
    std::thread::spawn(move || {
        let _ = tx.send(measure(&files, gpu));
    });
    match rx.recv_timeout(BUDGET) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => {
            info!(
                "ocr-daemon: the {} engine did not answer in {BUDGET:?}",
                device(gpu)
            );
            None
        }
        Err(RecvTimeoutError::Disconnected) => {
            info!(
                "ocr-daemon: the {} engine crashed while measuring",
                device(gpu)
            );
            None
        }
    }
}

fn device(gpu: bool) -> &'static str {
    if gpu { "GPU" } else { "CPU" }
}

fn measure(files: &ModelFiles, gpu: bool) -> Option<u32> {
    let built = if gpu {
        PaddleBackend::on_gpu(files)
    } else {
        PaddleBackend::new(files)
    };
    let backend = match built {
        Ok(backend) => backend,
        Err(e) => {
            info!(
                "ocr-daemon: no {} engine for the measurement: {e}",
                device(gpu)
            );
            return None;
        }
    };
    let mut best = None;
    for scan in 0..SCANS {
        let started = Instant::now();
        let read: Result<OcrText, _> =
            crate::ocr::OcrBackend::recognize(&backend, probe_image(), &|| false);
        let Ok(text) = read else {
            info!(
                "ocr-daemon: the {} engine could not read the probe image",
                device(gpu)
            );
            return None;
        };
        let ms = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
        info!(
            "ocr-daemon: probe on {} #{} took {ms} ms ({} lines)",
            device(gpu),
            scan + 1,
            text.lines.len()
        );
        // The first GPU scan pays for compiling shaders for these shapes; the warm one is what matters.
        best = Some(best.map_or(ms, |before: u32| before.min(ms)));
    }
    best
}

pub fn probe_image() -> OcrImage {
    let (width, height) = (PROBE_WIDTH, PROBE_HEIGHT);
    let mut rgb = vec![0xF4u8; (width as usize) * (height as usize) * 3];
    let mut seed: u32 = 0x5EED_1234;
    let mut next = |range: u32| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 16) % range
    };
    for line in 0..PROBE_LINES {
        let top = 24 + line * LINE_HEIGHT;
        let mut x = 40;
        while x < width - 80 {
            let glyph_w = 7 + next(7);
            let glyph_h = 18 + next(8);
            let y = top + (26 - glyph_h / 2);
            bar(&mut rgb, width, height, x, y, glyph_w, glyph_h);
            x += glyph_w + 5 + if next(7) == 0 { 16 } else { 0 };
        }
    }
    OcrImage {
        rgb,
        width,
        height,
        origin: (0.0, 0.0),
    }
}

fn bar(rgb: &mut [u8], width: u32, height: u32, x: u32, y: u32, w: u32, h: u32) {
    for row in y..(y + h).min(height) {
        for column in x..(x + w).min(width) {
            let at = ((row as usize) * (width as usize) + column as usize) * 3;
            rgb[at..at + 3].copy_from_slice(&[0x18, 0x18, 0x18]);
        }
    }
}
