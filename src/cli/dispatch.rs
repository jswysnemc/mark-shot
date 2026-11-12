use std::ffi::OsString;
use std::time::Instant;

use super::arguments::{self, Command};
use crate::AppResult;
use crate::backend::{clipboard, pin};
use crate::logging::Process;

/// 【命令行】【执行】按values解析系统字符串参数并调度任务，返回执行结果；帮助和辅助进程不创建截图运行时
pub fn run(values: impl IntoIterator<Item = OsString>) -> AppResult {
    let options = arguments::parse(values, Instant::now())?;
    match options.command {
        Command::Help => {
            print!("{}", super::help::HELP);
            return Ok(());
        }
        Command::Version => {
            println!("mark-shot {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Command::DefaultConfig => {
            print!("{}", crate::config::TEMPLATE);
            return Ok(());
        }
        _ => {}
    }

    // 1. 【内存生命周期】【启动顺序】创建字体和捕获线程前设置大对象策略，辅助进程也使用同一分配规则
    crate::utils::memory::configure();
    // 2. 【命令行】【运行配置】初始化配置后再启动日志，命令行覆盖只影响当前进程
    crate::config::runtime_overrides::record(
        options.no_notify.then_some(false),
        options
            .debug
            .map(|enabled| if enabled { "info" } else { "off" }.into()),
        options
            .debug_log
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        options
            .xdg_window
            .then_some(crate::backend::OverlayShell::Window),
    );
    crate::config::init(options.config.clone(), |config| {
        if options.no_notify {
            config.notifications.enabled = false;
        }
        if let Some(enabled) = options.debug {
            config.log.level = Some(if enabled { "info" } else { "off" }.into());
        }
        if let Some(path) = &options.debug_log {
            config.log.path = Some(path.to_string_lossy().into_owned());
        }
        if options.xdg_window {
            config.general.overlay = crate::backend::OverlayShell::Window;
        }
    });
    if let Command::Internal { kind, args } = options.command {
        return match kind.as_str() {
            crate::features::desktop::ARG => {
                if !args.is_empty() {
                    return Err("background service accepts no additional arguments".into());
                }
                crate::logging::init(Process::Overlay);
                crate::features::desktop::run()
            }
            crate::features::preferences::READY_ARG => {
                if !args.is_empty() {
                    return Err("settings window accepts no internal arguments".into());
                }
                crate::logging::init(Process::Overlay);
                crate::features::preferences::run(true)
            }
            clipboard::DAEMON_ARG => {
                crate::logging::init(Process::Clipboard);
                clipboard::daemon::cli(args.into_iter());
                Ok(())
            }
            pin::PIN_ARG => {
                crate::logging::init(Process::Pin);
                pin::cli(args.into_iter())
            }
            crate::ocr::daemon::DAEMON_ARG => {
                crate::logging::init(Process::OcrDaemon);
                crate::ocr::daemon::cli(args.into_iter())
            }
            crate::features::ocr_result::transport::RESULT_ARG => {
                crate::logging::init(Process::OcrResult);
                crate::features::ocr_result::transport::cli(&args)
            }
            crate::features::ocr_result::floating::ARG => {
                crate::logging::init(Process::OcrResult);
                crate::features::ocr_result::floating::cli(&args)
            }
            crate::features::code_scan::transport::ARG => {
                crate::logging::init(Process::Overlay);
                crate::features::code_scan::transport::cli(&args)
            }
            crate::features::pinned_image::recognition::WORKER_ARG => {
                crate::logging::init(Process::OcrDaemon);
                crate::features::pinned_image::recognition::cli(&args)
            }
            crate::features::capture_history::transport::READY_ARG => {
                if !args.is_empty() {
                    return Err("history window accepts no internal arguments".into());
                }
                crate::logging::init(Process::History);
                crate::config::log_warnings();
                crate::features::capture_history::run(true)
            }
            _ => Err("unknown internal command".into()),
        };
    }
    if matches!(options.command, Command::Settings) {
        if let Err(error) = crate::features::desktop::ensure() {
            log::error!("【桌面集成】【服务启动】{error}");
        }
        crate::logging::init(Process::Overlay);
        return crate::features::preferences::run(false);
    }
    if let Command::CodeScan { path, json } = &options.command {
        crate::logging::init(Process::Overlay);
        return crate::features::code_scan::transport::file(path, *json);
    }
    if let Command::Recording(target) = options.command {
        crate::logging::init(Process::Overlay);
        crate::features::desktop::ensure()?;
        return crate::features::recording::run(target);
    }
    if let Command::RecordingControl(action) = &options.command {
        return crate::features::recording::command(action);
    }
    if matches!(options.command, Command::History) {
        crate::logging::init(Process::History);
        crate::config::log_warnings();
        return crate::features::capture_history::run(false);
    }
    crate::logging::init(Process::Overlay);
    if !matches!(
        options.command,
        Command::ListDisplays | Command::Capture { .. }
    ) && let Err(error) = crate::features::desktop::ensure()
    {
        log::error!("【桌面集成】【服务启动】{error}");
    }
    match options.command {
        Command::PinFile(path) => return pin::run(&std::fs::read(path)?, 1.0),
        Command::PinClipboard => return super::pin_clipboard::run(),
        Command::ListDisplays => return super::headless::list_displays(),
        _ => {}
    }
    let (_history_image, command) = match options.command {
        Command::HistoryEdit(name) => {
            let (file, launch) = crate::features::capture_history::transport::edit_snapshot(&name)?;
            (Some(file), Command::Editor(launch))
        }
        Command::PinOcrSnapshot { copy } => {
            let (file, mut launch) =
                crate::features::capture_history::transport::edit_from_stdin()?;
            launch.mode = crate::types::Mode::Ocr;
            launch.to = Some(crate::types::Outputs {
                copy,
                ..Default::default()
            });
            (Some(file), Command::Editor(launch))
        }
        Command::HistorySnapshot => {
            let (file, launch) = crate::features::capture_history::transport::edit_from_stdin()?;
            (Some(file), Command::Editor(launch))
        }
        command => (None, command),
    };

    // 3. 【截图运行时】【任务调度】阻塞任务结束后释放空闲线程，交互会话不保留十秒工作线程
    let runtime = super::capture_runtime::build()?;
    runtime.block_on(async {
        match command {
            Command::Capture(mut capture) => {
                capture.include_cursor |= crate::config::get().startup.include_cursor;
                super::headless::run(&capture).await
            }
            Command::Editor(mut launch) => {
                crate::config::get().startup.apply(&mut launch);
                let connection = crate::backend::session::Session::connect_to_env()
                    .map_err(|error| format!("cannot connect to Wayland: {error}"))?;
                let result = super::single_instance::run(connection, launch).await;
                if let Err(error) = &result {
                    log::error!("【MarkShot】【截图】截图会话失败: {error}");
                    crate::backend::notify::send(crate::backend::notify::Notice::Failed(
                        error.to_string(),
                    ))
                    .await;
                }
                result
            }
            _ => Ok(()),
        }
    })
}
