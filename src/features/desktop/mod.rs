//! 【桌面集成】【常驻服务】托盘和全局按键独立注册，配置变化后释放旧资源再建立新注册

mod actions;
mod delayed_capture;
mod hotkeys;
#[cfg(target_os = "windows")]
mod native_thread;
pub mod recording;
pub mod status;
mod tray;

use crate::config::desktop::Desktop;
use actions::Action;
use notify::Watcher;
use std::{fs::OpenOptions, path::Path};
use tokio::sync::{mpsc, watch};

pub const ARG: &str = "--background";

enum Event {
    Reload,
    RecordingPoll,
    DelayDue,
    Action(Action, Option<String>),
    Status(u64, bool, status::Registration),
}

#[derive(Clone)]
struct Reporter {
    sender: mpsc::UnboundedSender<Event>,
    generation: u64,
    hotkey: bool,
}

impl Reporter {
    /// 【桌面集成】【结果通知】按message通知当前注册状态，旧代次结果由服务丢弃，无返回值
    fn status(&self, message: status::Registration) {
        let _ = self
            .sender
            .send(Event::Status(self.generation, self.hotkey, message));
    }
    /// 【桌面集成】【操作通知】按action提交用户动作，不在系统回调内创建窗口，无返回值
    fn action(&self, action: Action) {
        self.activated(action, None);
    }
    /// 【桌面集成】【激活交付】按action和token向窗口交付系统焦点凭据，无返回值
    fn activated(&self, action: Action, token: Option<String>) {
        let _ = self.sender.send(Event::Action(action, token));
    }
}

struct Worker {
    stop: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}
impl Worker {
    /// 【桌面集成】【注册释放】消费句柄并等待原生资源释放，无参数，停止请求可取消门户授权等待
    async fn close(self) {
        let _ = self.stop.send(true);
        if let Err(error) = self.task.await {
            log::error!("【桌面集成】【停止注册】{error}");
        }
    }
}

/// 【桌面集成】【按需启动】无参数，按已保存配置启动独立服务，已运行时不创建重复进程
pub fn ensure() -> crate::AppResult {
    let config = crate::config::preferences::current();
    if !config.desktop.enabled() {
        return Ok(());
    }
    let root = status::directory(&crate::config::preferences::path()?)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join("service.lock"))?;
    match lock.try_lock() {
        Ok(()) => {
            drop(lock);
            crate::utils::spawn_self(&[ARG], &[])
        }
        Err(std::fs::TryLockError::WouldBlock) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// 【桌面集成】【服务入口】无参数，持有会话锁并驱动异步注册直到退出，返回启动错误
pub fn run() -> crate::AppResult {
    let path = std::path::absolute(crate::config::preferences::path()?)?;
    let root = status::directory(&path)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join("service.lock"))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => return Ok(()),
        Err(error) => return Err(error.into()),
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(serve(&path, &root))
}

/// 【桌面集成】【事件循环】按path和root加载配置，阻塞等待配置通知或桌面动作，返回资源错误
async fn serve(path: &Path, root: &Path) -> crate::AppResult {
    let parent = path.parent().ok_or("configuration has no parent")?;
    std::fs::create_dir_all(parent)?;
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let changes = sender.clone();
    let watched = path.to_path_buf();
    let recording_path = root.join(recording::FILE);
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.is_ok_and(|event| {
            (event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove())
                && event
                    .paths
                    .iter()
                    .any(|path| path == &watched || path == &recording_path)
        }) {
            let _ = changes.send(Event::Reload);
        }
    })?;
    watcher.watch(parent, notify::RecursiveMode::NonRecursive)?;
    watcher.watch(root, notify::RecursiveMode::NonRecursive)?;
    let mut current: Option<Desktop> = None;
    let mut workers: [Option<Worker>; 2] = [None, None];
    let mut generations = [0, 0];
    let mut paused = false;
    let mut status = status::Status::default();
    let mut delayed = delayed_capture::Schedule::default();
    let mut recording_query = crate::features::recording::status_query::Query::default();
    let (menu_state, _) = watch::channel(tray::menu::State {
        language: crate::ui::language::Language::current(),
        delayed: false,
        recording: tray::recording::State::Unavailable,
    });
    sender.send(Event::Reload)?;
    loop {
        let event = tokio::select! {
            biased;
            event = receiver.recv() => match event { Some(event) => event, None => break },
            _ = delayed.wait() => Event::DelayDue,
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)), if paused => Event::Reload,
            _ = tokio::time::sleep(recording_query.timeout()),
                if current.as_ref().is_some_and(|desktop| desktop.tray_enabled) => Event::RecordingPoll,
        };
        match event {
            Event::RecordingPoll => {
                if let Some(result) = recording_query.poll() {
                    let next = match result {
                        Ok(status) => tray::recording::State::from_status(&status),
                        Err(error) => {
                            log::error!("【系统托盘】【录制状态】{error}");
                            tray::recording::State::Unavailable
                        }
                    };
                    menu_state.send_if_modified(|state| {
                        if state.recording == next {
                            return false;
                        }
                        state.recording = next;
                        true
                    });
                }
            }
            Event::Reload => {
                let config = match crate::config::preferences::load(path) {
                    Ok(config) => config,
                    Err(error) => {
                        log::error!("【桌面集成】【读取配置】{error}");
                        continue;
                    }
                };
                let language = crate::ui::language::Language::resolve(
                    std::env::var("MARK_SHOT_LANG").ok().as_deref(),
                    config.ui.mode(config.language.as_deref()),
                    sys_locale::get_locale().as_deref(),
                );
                menu_state.send_if_modified(|state| {
                    if state.language == language {
                        return false;
                    }
                    state.language = language;
                    true
                });
                let next = config.desktop;
                if !next.tray_enabled {
                    delayed.cancel();
                }
                let recording = recording::active(root);
                if current.as_ref() == Some(&next) && recording == paused {
                    continue;
                }
                for (index, hotkey) in [true, false].into_iter().enumerate() {
                    let changed = current.as_ref().is_none_or(|previous| {
                        if hotkey {
                            previous.shortcuts_enabled != next.shortcuts_enabled
                                || previous.screenshot_shortcut != next.screenshot_shortcut
                                || recording != paused
                        } else {
                            previous.tray_enabled != next.tray_enabled
                        }
                    });
                    if !changed {
                        continue;
                    }
                    if let Some(worker) = workers[index].take() {
                        worker.close().await;
                    }
                    generations[index] += 1;
                    let enabled = if hotkey {
                        next.shortcuts_enabled
                    } else {
                        next.tray_enabled
                    };
                    let registration = if hotkey {
                        &mut status.shortcuts
                    } else {
                        &mut status.tray
                    };
                    *registration = if enabled {
                        status::Registration::Registering
                    } else {
                        status::Registration::Disabled
                    };
                    if enabled && hotkey && recording {
                        *registration = status::Registration::Paused;
                        continue;
                    }
                    if !enabled {
                        continue;
                    }
                    let (stop, cancelled) = watch::channel(false);
                    let reporter = Reporter {
                        sender: sender.clone(),
                        generation: generations[index],
                        hotkey,
                    };
                    let shortcut = next.screenshot_shortcut.clone();
                    let menu = menu_state.subscribe();
                    let task = tokio::spawn(async move {
                        if hotkey {
                            hotkeys::run(shortcut, reporter, cancelled).await;
                        } else {
                            tray::run(reporter, cancelled, menu).await;
                        }
                    });
                    workers[index] = Some(Worker { stop, task });
                }
                status::write(root, &status)?;
                if !next.enabled() {
                    break;
                }
                current = Some(next);
                paused = recording;
            }
            Event::Status(version, hotkey, message)
                if version == generations[usize::from(!hotkey)] =>
            {
                if hotkey {
                    status.shortcuts = message;
                } else {
                    status.tray = message;
                }
                status::write(root, &status)?;
            }
            Event::Action(Action::Quit, _) => break,
            Event::Action(Action::Delay(seconds), _) => {
                delayed.replace(seconds, std::time::Instant::now())
            }
            Event::Action(Action::CancelDelay, _) => delayed.cancel(),
            Event::DelayDue => {
                if delayed.take_due(std::time::Instant::now())
                    && let Err(error) = crate::utils::spawn_self_activated(
                        Action::Capture.arguments().unwrap(),
                        None,
                    )
                {
                    log::error!("【桌面集成】【延时截图】{error}");
                }
            }
            Event::Action(action, token) => {
                if matches!(action, Action::Capture | Action::Fullscreen) {
                    delayed.cancel();
                }
                let Some(args) = action.arguments() else {
                    continue;
                };
                if let Err(error) = crate::utils::spawn_self_activated(args, token.as_deref()) {
                    log::error!("【桌面集成】【启动操作】{error}");
                }
            }
            _ => {}
        }
        menu_state.send_if_modified(|state| {
            let pending = delayed.pending();
            if state.delayed == pending {
                return false;
            }
            state.delayed = pending;
            true
        });
    }
    delayed.cancel();
    for worker in workers.into_iter().flatten() {
        worker.close().await;
    }
    status::write(
        root,
        &status::Status {
            shortcuts: status::Registration::Stopped,
            tray: status::Registration::Stopped,
        },
    )?;
    Ok(())
}
