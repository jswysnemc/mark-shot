use std::ffi::OsString;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::AppResult;
use crate::app::Launch;
use crate::theme::Rgba;
use crate::types::{Mode, Outputs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Region {
    /// 【命令行】【区域参数】解析全局逻辑坐标 x,y,width,height；拒绝空区域和溢出
    pub fn parse(text: &str) -> Result<Self, String> {
        let fields: Vec<_> = text.split(',').map(str::trim).collect();
        if fields.len() != 4 {
            return Err("--region expects x,y,width,height in logical pixels".into());
        }
        let x = fields[0].parse::<i32>().map_err(|_| "invalid region x")?;
        let y = fields[1].parse::<i32>().map_err(|_| "invalid region y")?;
        let width = fields[2]
            .parse::<u32>()
            .map_err(|_| "invalid region width")?;
        let height = fields[3]
            .parse::<u32>()
            .map_err(|_| "invalid region height")?;
        if width == 0 || height == 0 {
            return Err("region width and height must be positive".into());
        }
        let right = i64::from(x) + i64::from(width);
        let bottom = i64::from(y) + i64::from(height);
        if right > i64::from(i32::MAX) || bottom > i64::from(i32::MAX) {
            return Err("region coordinates overflow".into());
        }
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    /// 【命令行】【区域几何】将参数转换为绘制矩形；无法表示时返回空值
    pub fn rect(self) -> Option<tiny_skia::Rect> {
        tiny_skia::Rect::from_xywh(
            self.x as f32,
            self.y as f32,
            self.width as f32,
            self.height as f32,
        )
    }
}

#[derive(Debug, Clone)]
pub struct CaptureOptions {
    pub destination: PathBuf,
    pub displays: Vec<String>,
    pub all_outputs: bool,
    pub region: Option<Region>,
    pub include_cursor: bool,
    pub output_name: Option<String>,
    pub shot_at: Option<Instant>,
    pub speed: bool,
}

#[derive(Debug)]
pub enum Command {
    Help,
    Version,
    DefaultConfig,
    Editor(Launch),
    Capture(CaptureOptions),
    ListDisplays,
    History,
    Settings,
    Recording(crate::features::recording::Target),
    RecordingControl(String),
    HistoryEdit(String),
    HistorySnapshot,
    PinOcrSnapshot { copy: bool },
    PinFile(PathBuf),
    PinClipboard,
    Internal { kind: String, args: Vec<String> },
}

#[derive(Debug)]
pub struct Arguments {
    pub command: Command,
    pub config: Option<PathBuf>,
    pub no_notify: bool,
    pub debug: Option<bool>,
    pub debug_log: Option<PathBuf>,
    pub xdg_window: bool,
}

/// 【命令行】【延迟参数】把 text 秒数转换为原版零至六十秒范围内的有效时长，返回非法值错误
fn delay(text: &str) -> Result<Duration, &'static str> {
    text.parse::<f64>()
        .ok()
        .filter(|value| (0.0..=60.0).contains(value))
        .and_then(|value| Duration::try_from_secs_f64(value).ok())
        .ok_or("--delay expects 0 to 60 seconds, e.g. 0.5")
}

/// 【命令行】【路径参数】保留系统路径编码，不把非 UTF-8 文件名转换为空字符串
fn path(args: &mut pico_args::Arguments, flag: &'static str) -> AppResult<Option<PathBuf>> {
    Ok(args.opt_value_from_os_str(flag, |value| Ok::<_, String>(PathBuf::from(value)))?)
}

/// 【命令行】【解析】将系统参数转换为一个应用命令；冲突、非法值和未知选项返回错误
pub fn parse(values: impl IntoIterator<Item = OsString>, started: Instant) -> AppResult<Arguments> {
    let mut args = pico_args::Arguments::from_vec(values.into_iter().collect());
    let help = args.contains(["-h", "--help"]);
    let version = args.contains(["-v", "--version"]) || args.contains("-V");
    let defaults = args.contains("--print-default-config");
    if help || version || defaults {
        return Ok(Arguments {
            command: if help {
                Command::Help
            } else if version {
                Command::Version
            } else {
                Command::DefaultConfig
            },
            config: None,
            no_notify: false,
            debug: None,
            debug_log: None,
            xdg_window: false,
        });
    }

    // 1. 先读取所有进程共用的配置，内部进程不初始化图形或异步运行时
    let config = path(&mut args, "--config")?;
    let no_notify = args.contains(["-n", "--nonotify"]);
    let debug_log = path(&mut args, "--debug-log")?;
    let enabled = args.contains("--debug");
    let disabled = args.contains("--no-debug");
    let debug = if disabled {
        Some(false)
    } else if enabled || debug_log.is_some() {
        Some(true)
    } else {
        None
    };
    let xdg_window = args.contains("--xdg-window");
    for kind in [
        crate::features::desktop::ARG,
        crate::backend::clipboard::DAEMON_ARG,
        crate::backend::pin::PIN_ARG,
        crate::ocr::daemon::DAEMON_ARG,
        crate::features::ocr_result::transport::RESULT_ARG,
        crate::features::ocr_result::floating::ARG,
        crate::features::capture_history::transport::READY_ARG,
        crate::features::preferences::READY_ARG,
        crate::features::pinned_image::recognition::WORKER_ARG,
    ] {
        if args.contains(kind) {
            let rest = args
                .finish()
                .into_iter()
                .map(|value| {
                    value
                        .into_string()
                        .map_err(|_| "internal command expects UTF-8 arguments")
                })
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(Arguments {
                command: Command::Internal {
                    kind: kind.into(),
                    args: rest,
                },
                config,
                no_notify,
                debug,
                debug_log,
                xdg_window,
            });
        }
    }

    // 2. 解析捕获范围和编辑行为，最终只允许执行一种主命令
    let capture_to = path(&mut args, "--capture-to")?;
    let list_displays = args.contains("--list-displays");
    let history = args.contains("--history");
    let settings = args.contains("--settings");
    let recording = args.contains("--record");
    let record_select = args.contains("--record-select");
    let record_target: Option<String> = args.opt_value_from_str("--record-target")?;
    let controls: Vec<_> = [
        ("--recording-status", "status"),
        ("--pause-recording", "pause"),
        ("--resume-recording", "resume"),
        ("--stop-recording", "stop"),
    ]
    .into_iter()
    .filter_map(|(flag, action)| args.contains(flag).then_some(action))
    .collect();
    if controls.len() > 1 || (record_target.is_some() && !recording) {
        return Err("conflicting recording options / 录制参数冲突".into());
    }
    let history_edit: Option<String> = args.opt_value_from_str("--history-edit")?;
    let history_snapshot = args.contains("--history-edit-snapshot");
    let pin_ocr_copy = args.contains(crate::features::pinned_image::actions::OCR_COPY_ARG);
    let pin_ocr_snapshot =
        args.contains(crate::features::pinned_image::actions::OCR_ARG) || pin_ocr_copy;
    if history_edit
        .as_deref()
        .is_some_and(|name| crate::features::capture_history::identity::timestamp(name).is_none())
    {
        return Err("--history-edit requires a valid history filename".into());
    }
    let pin_image = path(&mut args, "--pin-image")?;
    let pin_clipboard = args.contains("--pin-clipboard");
    let full = args.contains(["-f", "--full"]);
    let window = args.contains(["-w", "--window"]);
    let quick_region = args.contains(["-r", "--select-region"]) || record_select;
    let ocr = args.contains("--ocr");
    let scroll = args.contains(["-S", "--scroll"]);
    let fullscreen = args.contains("--fullscreen") | args.contains("--full-screen");
    let all_outputs = args.contains("--all-outputs") | args.contains("--all-output");
    let monitor = args.contains(["-m", "--monitor"]);
    let _capture = args.contains("--capture");
    let include_cursor = args.contains("--include-cursor");
    let speed = args.contains(["-s", "--speed"]);
    let output = path(&mut args, "--output")?;
    let region = args.opt_value_from_fn("--region", Region::parse)?;
    let output_name: Option<String> = args.opt_value_from_str("--output-name")?;
    let shot_at = args
        .opt_value_from_fn(["-d", "--delay"], delay)?
        .map(|delay| {
            started
                .checked_add(delay)
                .ok_or("capture delay is too long")
        })
        .transpose()?;
    let mut displays = Vec::new();
    while let Some(display) = args.opt_value_from_str::<_, String>("--display")? {
        if display.is_empty() {
            return Err("--display must not be empty".into());
        }
        if !displays.contains(&display) {
            displays.push(display);
        }
    }
    let to = match args.opt_value_from_fn(["-t", "--to"], Outputs::parse) {
        Err(pico_args::Error::OptionWithoutAValue(_)) => {
            args.contains(["-t", "--to"]);
            Some(Outputs::default())
        }
        value => value?,
    };
    let default_tool = args.opt_value_from_fn("--default-tool", super::tool_names::parse)?;
    let fullscreen_default_tool =
        args.opt_value_from_fn("--fullscreen-default-tool", super::tool_names::parse)?;
    let file_default_tool =
        args.opt_value_from_fn("--file-default-tool", super::tool_names::parse)?;
    let default_color = args.opt_value_from_fn("--default-color", |value| {
        Rgba::parse_hex(value).ok_or("expected #RRGGBB or #RRGGBBAA")
    })?;
    let mut rest = args.finish();
    if let Some(option) = rest
        .iter()
        .find(|value| value.to_string_lossy().starts_with('-'))
    {
        return Err(format!(
            "unknown or unavailable option: {}",
            option.to_string_lossy()
        )
        .into());
    }
    let quick_ocr = quick_region && rest.first().is_some_and(|value| value == "ocr");
    if quick_ocr {
        rest.remove(0);
    }
    if rest.len() > 1 {
        return Err("only one image file may be opened".into());
    }
    let image = rest.pop().map(PathBuf::from);
    let modes = [full, window, quick_region, ocr, scroll]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();
    if modes > 1 {
        return Err("pick one capture mode".into());
    }
    let commands = [
        capture_to.is_some(),
        list_displays,
        history,
        settings,
        recording,
        !controls.is_empty(),
        history_edit.is_some(),
        history_snapshot,
        pin_ocr_snapshot,
        pin_image.is_some(),
        pin_clipboard,
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count();
    if commands > 1 || (commands > 0 && (image.is_some() || modes > 0 || fullscreen)) {
        return Err(
            "capture, history, display listing, pinning and image editing are mutually exclusive"
                .into(),
        );
    }
    if region.is_some() && capture_to.is_none() {
        return Err("--region requires --capture-to".into());
    }
    if output_name.is_some() && capture_to.is_none() {
        return Err("--output-name requires --capture-to".into());
    }
    if monitor && all_outputs {
        return Err("--monitor and --all-outputs are mutually exclusive".into());
    }
    if (history || settings)
        && (output.is_some()
            || to.is_some()
            || default_tool.is_some()
            || fullscreen_default_tool.is_some()
            || file_default_tool.is_some()
            || default_color.is_some()
            || !displays.is_empty()
            || shot_at.is_some()
            || all_outputs
            || include_cursor)
    {
        return Err("--history cannot be combined with capture or editor options".into());
    }
    let mode = if full {
        Mode::Full
    } else if window {
        Mode::Window
    } else if ocr || quick_ocr {
        Mode::Ocr
    } else if scroll {
        Mode::Scroll
    } else if quick_region {
        Mode::Region
    } else {
        Mode::Editor
    };
    let command = if let Some(destination) = capture_to {
        if output.is_some() || to.is_some() {
            return Err("--capture-to cannot be combined with --output or --to".into());
        }
        Command::Capture(CaptureOptions {
            destination,
            displays,
            all_outputs,
            region,
            include_cursor,
            output_name,
            shot_at,
            speed,
        })
    } else if recording {
        Command::Recording(
            record_target
                .map(|value| serde_json::from_str(&value))
                .transpose()?
                .unwrap_or_default(),
        )
    } else if let Some(action) = controls.first() {
        Command::RecordingControl((*action).into())
    } else if list_displays {
        Command::ListDisplays
    } else if settings {
        Command::Settings
    } else if history {
        Command::History
    } else if let Some(name) = history_edit {
        Command::HistoryEdit(name)
    } else if history_snapshot {
        Command::HistorySnapshot
    } else if pin_ocr_snapshot {
        Command::PinOcrSnapshot { copy: pin_ocr_copy }
    } else if let Some(path) = pin_image {
        Command::PinFile(path)
    } else if pin_clipboard {
        Command::PinClipboard
    } else {
        Command::Editor(Launch {
            mode,
            one_monitor: monitor || (full && !all_outputs),
            all_outputs,
            to,
            output,
            speed,
            shot_at,
            image,
            fullscreen,
            display_names: displays,
            include_cursor,
            default_tool,
            fullscreen_default_tool,
            file_default_tool,
            default_color,
            record_select,
        })
    };
    Ok(Arguments {
        command,
        config,
        no_notify,
        debug,
        debug_log,
        xdg_window,
    })
}

#[cfg(test)]
#[path = "arguments/startup_tests.rs"]
mod startup_tests;

#[cfg(test)]
mod delay_tests {
    use super::*;

    /// 【命令行回归】【延迟范围】超过原版一分钟上限及非有限秒数在参数阶段拒绝，无参数、无返回值
    #[test]
    fn delay_outside_qt_range_is_rejected_before_graphics() {
        for value in ["60.000001", "61", "-0.5", "inf", "NaN"] {
            assert!(
                parse(
                    [OsString::from("--delay"), OsString::from(value)],
                    Instant::now()
                )
                .is_err(),
                "delay={value}"
            );
        }
    }

    /// 【命令行回归】【有效秒数】零、小数及一分钟边界保留准确期限，无参数、无返回值
    #[test]
    fn qt_delay_endpoints_and_fractional_seconds_are_accepted() {
        let now = Instant::now();
        for (value, duration) in [
            ("0", Duration::ZERO),
            ("0.5", Duration::from_millis(500)),
            ("60", Duration::from_secs(60)),
        ] {
            let options = parse([OsString::from("--delay"), OsString::from(value)], now).unwrap();
            let Command::Editor(launch) = options.command else {
                panic!("expected editor");
            };
            assert_eq!(launch.shot_at, now.checked_add(duration));
        }
    }
}
