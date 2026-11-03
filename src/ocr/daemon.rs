// Persistent OCR daemon keeping the recognition model ready in memory between overlay sessions.
// Enforces a single instance per user via `flock`, bound to a UNIX socket in `XDG_RUNTIME_DIR`.
// Exits automatically on idle timeout, explicit `Shutdown`, binary mismatch/upgrade, or engine failure.
// When unavailable, OCR falls back to executing in-process within the overlay.
//
// protocol - Socket wire format and framing primitives
// server   - Daemon service daemon entry point (`--ocr-daemon serve | status | stop`)
// client   - `OcrBackend` client wrapper that delegates to the daemon with automatic local fallback

pub mod calibrate;
pub mod client;
pub mod protocol;
pub mod server;
#[cfg(test)]
mod tests;

use log::{error, info};
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nix::fcntl::{Flock, FlockArg};

use super::dawn;
use super::models::ModelFiles;
use super::paddle_backend::PaddleBackend;
use super::settings::{Device, EngineSettings, GPU_BUILD, Mode};
use calibrate::{Record, Verdict};
use protocol::EngineState;

#[derive(Debug, Clone)]
pub struct Paths {
    pub dir: PathBuf,
    pub socket: PathBuf,
    pub lock: PathBuf,
    pub strikes: PathBuf,
}

impl Paths {
    pub fn from_env() -> Option<Self> {
        let dir = std::env::var_os("XDG_RUNTIME_DIR").filter(|dir| !dir.is_empty())?;
        Some(Self::in_dir(PathBuf::from(dir).join("mark-shot")))
    }

    pub fn in_dir(dir: PathBuf) -> Self {
        Self {
            socket: dir.join("ocr.sock"),
            lock: dir.join("ocr.lock"),
            strikes: dir.join("ocr.strikes"),
            dir,
        }
    }
}

/// Builds id & settings
/// A new running daemon with a different identity gets replaced
pub fn identity(settings: &EngineSettings) -> String {
    let idle = match settings.daemon_idle {
        Some(idle) => idle.as_secs(),
        None => 0,
    };
    format!("{}-{:?}-{idle}", build_id(), settings.device)
}

/// Application version along with binary size and modification timestamp
/// rebuilding or updating changes the ID.
pub fn build_id() -> &'static str {
    static ID: OnceLock<String> = OnceLock::new();
    ID.get_or_init(|| {
        let stamp = std::env::current_exe()
            .and_then(fs::metadata)
            .map(|meta| {
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_nanos());
                format!("{}-{mtime}", meta.len())
            })
            .unwrap_or_default();
        format!("{}-{stamp}", env!("CARGO_PKG_VERSION"))
    })
}

/// What the machine measured last time, if those numbers still apply to it.
pub fn measurement() -> Option<Record> {
    calibrate::stored(&calibrate::cache_path()?, &calibrate::machine_key())
}

pub fn save_measurement(record: &Record) {
    if let Some(path) = calibrate::cache_path() {
        calibrate::store(&path, record, &calibrate::machine_key());
    }
}

pub fn resolve_mode(settings: &EngineSettings) -> Mode {
    let paths = Paths::from_env();
    // local modes read neither strikes nor measurements nor /sys
    // daemon and GPU do not exist for them to keep them isolated to avoid bugs
    if settings.mode != Mode::Daemon {
        info!(
            "ocr: engine mode = {:?} (nothing stays in the background; mode = \"daemon\" in \
             ~/.config/mark-shot/config.toml's [ocr] section keeps a faster engine warm instead)",
            settings.mode
        );
        if let Some(paths) = paths
            && !lock_free(&paths.lock)
        {
            // daemon left over from mode = daemon would otherwise hold memory (and VRAM) until idling out
            std::thread::spawn(move || {
                if client::stop(&paths, Duration::from_secs(2), &client::kill_hard) == Ok(true) {
                    info!("ocr: stopped the daemon left from mode = daemon");
                }
            });
        }
        return settings.mode;
    }

    let ruled_out = paths
        .as_ref()
        .is_some_and(|paths| gpu_ruled_out(&read_strikes(paths), &identity(settings)));
    let measured = measurement();
    let cpu_won = measured.is_some_and(|record| record.verdict == Verdict::Cpu);
    let mode = settings.resolve(paths.is_some(), GPU_BUILD && !ruled_out && !cpu_won);

    if mode == settings.mode {
        let how = match (mode, measured) {
            (_, Some(record)) if settings.device == Device::Auto => match record.verdict {
                Verdict::Gpu => {
                    format!(
                        "{}, the daemon holds that engine in video memory",
                        record.summary()
                    )
                }
                Verdict::Cpu => record.summary(),
            },
            (_, None) if settings.device == Device::Auto && GPU_BUILD => {
                "first run, the daemon measures CPU against GPU once".to_owned()
            }
            _ => format!("device {:?}", settings.device),
        };
        let fetch =
            if mode == Mode::Daemon && settings.device != Device::Cpu && dawn::find().is_none() {
                format!(
                    "; the daemon downloads {} (14 MB) first, OCR runs here meanwhile",
                    dawn::LIBRARY
                )
            } else {
                String::new()
            };
        info!("ocr: engine mode = {mode:?} ({how}{fetch})");
    } else {
        let why = if paths.is_none() {
            "no XDG_RUNTIME_DIR".to_owned()
        } else if let Some(record) = measured.filter(|_| cpu_won) {
            format!("measured here: {}", record.summary())
        } else if ruled_out {
            "this binary already failed to put the engine on a GPU here".to_owned()
        } else {
            "this binary has no GPU engine compiled in yet (nothing to do with your graphics card)"
                .to_owned()
        };
        let hint = if cpu_won {
            "--ocr-daemon calibrate measures again"
        } else {
            "set device = \"cpu\" in ~/.config/mark-shot/config.toml's [ocr] section to run the daemon on the CPU anyway"
        };
        info!(
            "ocr: engine mode = {mode:?} (config wants {:?}; {why} -- {hint})",
            settings.mode
        );
    }
    if mode != Mode::Daemon
        && let Some(paths) = &paths
        && !lock_free(&paths.lock)
    {
        info!(
            "ocr: a daemon from an earlier run is still up, this mode does not use it (--ocr-daemon stop)"
        );
    }
    mode
}

pub fn model_name(files: &ModelFiles) -> String {
    files.recognizer.to_string_lossy().into_owned()
}

pub fn engine_factory(device: Device, paths: Paths) -> server::Factory {
    Arc::new(move |files: &ModelFiles| match device {
        Device::Cpu => cpu_engine(files),
        Device::Gpu if GPU_BUILD => gpu_engine(&paths, files),
        Device::Auto if GPU_BUILD => match measurement().map(|record| record.verdict) {
            Some(Verdict::Gpu) => gpu_engine(&paths, files),
            Some(Verdict::Cpu) => Err(server::BuildError::NoGpu),
            None if let Err(e) = gpu_library() => Err(e),
            None => match calibrate::run(files, &save_measurement).verdict {
                Verdict::Gpu => gpu_engine(&paths, files),
                Verdict::Cpu => Err(server::BuildError::NoGpu),
            },
        },
        Device::Auto | Device::Gpu => Err(server::BuildError::NoGpu),
    })
}

fn cpu_engine(files: &ModelFiles) -> Result<server::Engine, server::BuildError> {
    PaddleBackend::new(files)
        .map(|backend| server::Engine {
            backend: Box::new(backend),
            device: "cpu".into(),
        })
        .map_err(|e| server::BuildError::Failed(e.to_string()))
}

// failed load is marked as Failed, not NoGpu
// so without network, it ll try again later rather than waiting until reboot
fn gpu_library() -> Result<(), server::BuildError> {
    dawn::ensure().map_err(|e| {
        error!("ocr-daemon: {e}");
        server::BuildError::Failed(e)
    })
}

fn gpu_engine(paths: &Paths, files: &ModelFiles) -> Result<server::Engine, server::BuildError> {
    gpu_library()?;
    // fixxxx, segfault in the driver cannot be caught by catch_unwind
    // next daemon learns about it from the leftover file
    let marker = paths.dir.join("ocr.gpu-loading");
    let pid = std::process::id().to_string();
    if fs::read_to_string(&marker).is_ok_and(|owner| owner.trim() != pid) {
        let _ = fs::remove_file(&marker);
        info!(
            "ocr-daemon: the previous daemon died while putting the engine on the GPU, not trying again"
        );
        return Err(server::BuildError::NoGpu);
    }
    let _ = fs::write(&marker, &pid);
    let built = PaddleBackend::on_gpu(files)
        .map(|backend| server::Engine {
            backend: Box::new(backend),
            device: "webgpu".into(),
        })
        .map_err(|e| server::BuildError::Failed(e.to_string()));
    let _ = fs::remove_file(&marker);
    built
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strike {
    NoGpu,
    Failed,
}

const STRIKE_LIMIT: usize = 3;
const STRIKE_WINDOW_SECS: u64 = 600;
const STRIKES_KEPT: usize = 8;

pub fn record_strike(paths: &Paths, build: &str, strike: Strike) {
    let old = read_strikes(paths);
    let mut lines: Vec<&str> = old.lines().collect();
    let word = match strike {
        Strike::NoGpu => "no-gpu",
        Strike::Failed => "failed",
    };
    let line = format!("{} {word} {build}", unix_now());
    lines.push(&line);
    let text = lines[lines.len().saturating_sub(STRIKES_KEPT)..].join("\n") + "\n";
    if let Err(e) = fs::create_dir_all(&paths.dir).and_then(|()| fs::write(&paths.strikes, text)) {
        info!("ocr: cannot record a daemon failure: {e}");
    }
}

pub fn read_strikes(paths: &Paths) -> String {
    fs::read_to_string(&paths.strikes).unwrap_or_default()
}

pub fn clear_strikes(paths: &Paths) {
    if !read_strikes(paths).is_empty() {
        let _ = fs::remove_file(&paths.strikes);
    }
}

/// Do not spawn the daemo. Cause build lacks GPU support, or the daemon has crashed 3 times within 10 minutes.
pub fn daemon_blocked(strikes: &str, build: &str, now: u64) -> bool {
    let mut failures = 0;
    for (time, strike) in strikes_of(strikes, build) {
        match strike {
            Strike::NoGpu => return true,
            Strike::Failed if now.saturating_sub(time) < STRIKE_WINDOW_SECS => failures += 1,
            Strike::Failed => {}
        }
    }
    failures >= STRIKE_LIMIT
}

pub fn gpu_ruled_out(strikes: &str, build: &str) -> bool {
    strikes_of(strikes, build).any(|(_, strike)| strike == Strike::NoGpu)
}

fn strikes_of<'a>(strikes: &'a str, build: &'a str) -> impl Iterator<Item = (u64, Strike)> + 'a {
    strikes.lines().filter_map(move |line| {
        let mut parts = line.splitn(3, ' ');
        let time = parts.next()?.parse().ok()?;
        let strike = match parts.next()? {
            "no-gpu" => Strike::NoGpu,
            "failed" => Strike::Failed,
            _ => return None,
        };
        (parts.next()? == build).then_some((time, strike))
    })
}

pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Lock is free implies daemon is inactive
/// kernel automatically releases flock upon process termination.
pub fn lock_free(path: &Path) -> bool {
    match OpenOptions::new().read(true).open(path) {
        Ok(file) => Flock::lock(file, FlockArg::LockExclusiveNonblock).is_ok(),
        Err(_) => true,
    }
}

pub fn wait_lock_free(path: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if lock_free(path) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub fn lock_file_pid(paths: &Paths) -> Option<i32> {
    fs::read_to_string(&paths.lock).ok()?.trim().parse().ok()
}

pub fn spawn_daemon() -> io::Result<()> {
    let mut command = Command::new(std::env::current_exe()?);
    command.args([DAEMON_ARG, "serve"]);
    crate::config::pass_to(&mut command);
    spawn_detached(command, crate::logging::open_log_file())
}

/// 【文字识别】【进程启动】配置 command 的空标准流和 log，再独立启动；父进程不保留等待线程
pub fn spawn_detached(mut command: Command, log: Option<std::fs::File>) -> io::Result<()> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log.map_or_else(Stdio::null, Stdio::from));
    crate::utils::helper_process::spawn(command).map(|_| ())
}

/// Arguments used by the overlay to re-run itself as the daemon
pub const DAEMON_ARG: &str = "--ocr-daemon";

pub fn cli(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn Error>> {
    let paths = Paths::from_env()
        .ok_or("XDG_RUNTIME_DIR is not set, the OCR daemon has nowhere to live")?;
    match args.next().as_deref() {
        None | Some("serve") => serve(paths),
        Some("status") => status(&paths),
        Some("stop") => stop(&paths),
        Some("calibrate") => recalibrate(&paths),
        Some(other) => Err(format!(
            "unknown command `{other}`, expected serve | status | stop | calibrate"
        )
        .into()),
    }
}

fn recalibrate(paths: &Paths) -> Result<(), Box<dyn Error>> {
    let models = crate::ocr::models::OcrModels::load();
    let files = models
        .active()
        .and_then(|idx| models.files(idx))
        .ok_or("no OCR model is installed yet, install one in the overlay first")?;
    if GPU_BUILD {
        dawn::ensure()?;
    }
    let record = calibrate::run(&files, &save_measurement);
    clear_strikes(paths);
    println!("ocr: {}", record.summary());
    if let Some(path) = calibrate::cache_path() {
        println!("saved to {}", path.display());
    }
    if !lock_free(&paths.lock) {
        println!("a daemon is still running with the previous decision (--ocr-daemon stop)");
    }
    Ok(())
}

fn serve(paths: Paths) -> Result<(), Box<dyn Error>> {
    let settings = EngineSettings::load();
    let config = server::Config::new(
        paths.clone(),
        identity(&settings),
        settings.daemon_idle,
        engine_factory(settings.device, paths.clone()),
    );
    server::serve(config)?;
    Ok(())
}

fn status(paths: &Paths) -> Result<(), Box<dyn Error>> {
    let Some(status) = client::status(paths) else {
        if lock_free(&paths.lock) {
            println!("ocr daemon: not running");
            std::process::exit(3);
        }
        let pid = lock_file_pid(paths).map_or("?".to_owned(), |pid| pid.to_string());
        println!(
            "ocr daemon: pid {pid} holds the lock but does not answer (try --ocr-daemon stop)"
        );
        std::process::exit(4);
    };
    let engine = match &status.state {
        EngineState::Idle => "idle, no model requested yet".to_owned(),
        EngineState::Loading { model } => format!("loading {}", file_name(model)),
        EngineState::Ready { device, model } => format!("ready on {device}, {}", file_name(model)),
        EngineState::Failed(e) => format!("failed: {e}"),
        EngineState::NoGpu => {
            "no GPU available, leaving (OCR runs in the overlay instead)".to_owned()
        }
    };
    println!(
        "ocr daemon: running, pid {}, up {}",
        status.pid,
        human(status.uptime_secs)
    );
    println!("engine:     {engine}");
    if let Some(record) = measurement() {
        println!("measured:   {}", record.summary());
    }
    if GPU_BUILD {
        match dawn::find() {
            Some(path) => println!("gpu lib:    {}", path.display()),
            None => println!(
                "gpu lib:    not downloaded yet, the daemon fetches it when it needs the GPU"
            ),
        }
    }
    println!(
        "served:     {} scan(s), last took {} ms",
        status.served, status.last_ms
    );
    println!("memory:     {} MB", status.rss_kb / 1024);
    match status.idle_left_secs {
        Some(secs) => println!("idle exit:  in {}", human(secs)),
        None => println!("idle exit:  never"),
    }
    println!("socket:     {}", paths.socket.display());
    if let Some(log) = crate::logging::log_file_path() {
        println!("log:        {}", log.display());
    }
    Ok(())
}

fn stop(paths: &Paths) -> Result<(), Box<dyn Error>> {
    if client::stop(paths, Duration::from_secs(2), &client::kill_hard)? {
        println!("ocr daemon: stopped");
    } else {
        println!("ocr daemon: not running");
    }
    Ok(())
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn human(secs: u64) -> String {
    match secs {
        0..60 => format!("{secs}s"),
        60..3600 => format!("{}m {}s", secs / 60, secs % 60),
        _ => format!("{}h {}m", secs / 3600, secs % 3600 / 60),
    }
}
