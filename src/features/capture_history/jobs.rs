//! 【截图历史】【后台任务】一次性工作线程与合并请求，任务完成后立即退出

use super::{Entry, Store, preview};
use crate::{
    backend::window::WindowWake,
    config::history::{History, Update},
    ui::file_picker::{Listing, save::ExistingFile},
};
use std::{
    io,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::JoinHandle,
};
use tiny_skia::Pixmap;

pub(super) enum Task {
    List,
    Preview(String),
    Copy(String),
    Pin(String),
    Edit(String),
    Delete(String),
    Clear,
    Prune(usize),
    Snapshot(String),
    Save {
        png: Arc<Vec<u8>>,
        path: PathBuf,
        expected: Option<ExistingFile>,
    },
    Directory {
        path: PathBuf,
        hidden: bool,
    },
    MakeFolder(PathBuf),
    Settings(Update),
}

impl Task {
    /// 【截图历史】【任务优先级】返回是否为主动操作；列表与预览允许后续请求替换
    pub fn foreground(&self) -> bool {
        !matches!(self, Self::List | Self::Preview(_) | Self::Directory { .. })
    }
    /// 【截图历史】【任务名称】返回短状态标签；不输出路径或截图文字
    fn label(&self) -> &'static str {
        match self {
            Self::List => "Refresh",
            Self::Preview(_) => "Preview",
            Self::Copy(_) => "Copy",
            Self::Pin(_) => "Pin",
            Self::Edit(_) => "Edit",
            Self::Delete(_) => "Delete",
            Self::Clear => "Clear",
            Self::Prune(_) => "Prune",
            Self::Snapshot(_) => "Save",
            Self::Save { .. } => "Save",
            Self::Directory { .. } => "Folder",
            Self::MakeFolder(_) => "New folder",
            Self::Settings(_) => "Settings",
        }
    }
}

pub(super) enum Response {
    Listed {
        path: PathBuf,
        result: io::Result<Vec<Entry>>,
    },
    Preview {
        name: String,
        result: io::Result<Pixmap>,
    },
    Snapshot {
        name: String,
        result: io::Result<Arc<Vec<u8>>>,
    },
    Directory {
        path: PathBuf,
        result: io::Result<Listing>,
    },
    Updated(io::Result<History>),
    Done {
        label: &'static str,
        result: io::Result<()>,
    },
}

struct Running {
    name: Option<String>,
    foreground: bool,
    cancelled: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

pub(super) struct Jobs {
    sender: Sender<Response>,
    receiver: Receiver<Response>,
    wake: Option<WindowWake>,
    running: Option<Running>,
    pending: Option<(Store, Task)>,
}

impl Default for Jobs {
    /// 【截图历史】【任务状态】创建响应通道但不建立线程；窗口接入唤醒后才执行任务
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver,
            wake: None,
            running: None,
            pending: None,
        }
    }
}

impl Jobs {
    /// 【截图历史】【任务接入】保存 wake，供已完成工作线程通知窗口；无返回值
    pub fn connect(&mut self, wake: WindowWake) {
        self.wake = Some(wake);
    }
    /// 【截图历史】【前台状态】返回是否已有主动操作；预览读取不阻止用户操作按钮
    pub fn foreground(&self) -> bool {
        self.running.as_ref().is_some_and(|job| job.foreground)
            || self
                .pending
                .as_ref()
                .is_some_and(|(_, task)| task.foreground())
    }
    /// 【截图历史】【预览去重】返回 name 是否已经在读取或排队；不增加周期检查
    pub fn preview_pending(&self, name: &str) -> bool {
        self.running.as_ref().and_then(|job| job.name.as_deref()) == Some(name)
            || matches!(&self.pending,Some((_,Task::Preview(key)))if key==name)
    }

    /// 【截图历史】【请求提交】提交 store 上的 task，至多一个工作线程；主动操作不能被后续预览覆盖
    pub fn request(&mut self, store: Store, task: Task) -> bool {
        if self.wake.is_none() || (task.foreground() && self.foreground()) {
            return false;
        }
        if let Some(running) = &self.running {
            if self
                .pending
                .as_ref()
                .is_some_and(|(_, pending)| pending.foreground())
                && !task.foreground()
            {
                return false;
            }
            if !running.foreground {
                running.cancelled.store(true, Ordering::Relaxed);
            }
            self.pending = Some((store, task));
            return true;
        }
        self.start(store, task);
        true
    }

    /// 【截图历史】【工作启动】在短期线程执行 task 并发送结果；store 固定本次目录，失败也通知窗口
    fn start(&mut self, store: Store, task: Task) {
        let sender = self.sender.clone();
        let wake = self.wake.as_ref().unwrap().clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancellation = cancelled.clone();
        let name = if let Task::Preview(name) = &task {
            Some(name.clone())
        } else {
            None
        };
        let foreground = task.foreground();
        let label = task.label();
        match std::thread::Builder::new()
            .name("capture-history".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    execute(&store, task, &cancellation)
                }))
                .unwrap_or_else(|_| Response::Done {
                    label,
                    result: Err(io::Error::other("history worker panicked")),
                });
                let _ = sender.send(result);
                wake.notify();
            }) {
            Ok(handle) => {
                self.running = Some(Running {
                    name,
                    foreground,
                    cancelled,
                    handle: Some(handle),
                })
            }
            Err(error) => {
                self.running = Some(Running {
                    name,
                    foreground,
                    cancelled,
                    handle: None,
                });
                let _ = self.sender.send(Response::Done {
                    label,
                    result: Err(error),
                });
                self.wake.as_ref().unwrap().notify();
            }
        }
    }

    /// 【截图历史】【完成接收】取出已经完成的一个任务并释放线程；返回响应，不阻塞等待新任务
    pub fn take(&mut self) -> Option<Response> {
        let response = self.receiver.try_recv().ok()?;
        if let Some(running) = self.running.take()
            && let Some(handle) = running.handle
        {
            let _ = handle.join();
        }
        if let Some((store, task)) = self.pending.take() {
            self.start(store, task);
        }
        Some(response)
    }
}

impl Drop for Jobs {
    /// 【截图历史】【资源释放】取消未完成预览并回收本实例线程；不保留后台轮询或孤立任务
    fn drop(&mut self) {
        if let Some(running) = self.running.take() {
            running.cancelled.store(true, Ordering::Relaxed);
            if let Some(handle) = running.handle {
                let _ = handle.join();
            }
        }
    }
}

/// 【截图历史】【后台执行】执行 store 上 task，cancelled 用于预览停止；返回类型化结果
fn execute(store: &Store, task: Task, cancelled: &AtomicBool) -> Response {
    let label = task.label();
    let result = match task {
        Task::List => {
            return Response::Listed {
                path: store.directory().to_path_buf(),
                result: store.entries(),
            };
        }
        Task::Preview(name) => {
            return Response::Preview {
                name: name.clone(),
                result: store
                    .open(&name)
                    .and_then(|file| preview::load(file, (800, 600), cancelled)),
            };
        }
        Task::Directory { path, hidden } => {
            return Response::Directory {
                result: Listing::read(&path, hidden),
                path,
            };
        }
        Task::Settings(update) => return Response::Updated(crate::config::history::update(update)),
        Task::Snapshot(name) => {
            return Response::Snapshot {
                name: name.clone(),
                result: store.read(&name).map(|(png, _)| Arc::new(png)),
            };
        }
        Task::Copy(name) => store.read(&name).and_then(|(png, _)| {
            crate::backend::initialize_clipboard()
                .copy_image_to_clipboard(&png)
                .map_err(|error| io::Error::other(error.to_string()))
        }),
        Task::Pin(name) => store.read(&name).and_then(|(png, scale)| {
            crate::backend::pin::spawn(&png, scale)
                .map_err(|error| io::Error::other(error.to_string()))
        }),
        Task::Edit(name) => super::transport::spawn_edit(store, &name),
        Task::Delete(name) => store.remove(&name),
        Task::Clear => store.clear(),
        Task::Prune(limit) => store.prune(limit),
        Task::Save {
            png,
            path,
            expected,
        } => crate::ui::file_picker::save::write(&png, &path, expected),
        Task::MakeFolder(path) => std::fs::create_dir(path),
    };
    Response::Done { label, result }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【截图历史测试】【响应等待】在三秒内接收 jobs 一个响应；返回类型化结果，超时直接失败
    fn response(jobs: &mut Jobs) -> Response {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if let Some(response) = jobs.take() {
                return response;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    /// 【截图历史测试】【线程回收】列表任务结束后回收线程，主动清空不能被后续预览覆盖
    #[test]
    fn foreground_jobs_are_not_overwritten_and_workers_exit() {
        let directory = tempfile::tempdir().unwrap();
        let store = Store::new(directory.path(), 1024 * 1024);
        let entry = store
            .append(&super::super::tests::png(16, 9), 1.0, "", 50)
            .unwrap();
        let mut jobs = Jobs::default();
        jobs.connect(WindowWake::detached().unwrap());
        assert!(jobs.request(store.clone(), Task::List));
        assert!(matches!(
            response(&mut jobs),
            Response::Listed { result: Ok(_), .. }
        ));
        assert!(jobs.running.is_none());
        assert!(jobs.request(store.clone(), Task::Preview(entry.name.clone())));
        assert!(jobs.request(store.clone(), Task::Clear));
        assert!(!jobs.request(store.clone(), Task::Preview(entry.name)));
        response(&mut jobs);
        assert!(matches!(
            response(&mut jobs),
            Response::Done {
                label: "Clear",
                result: Ok(())
            }
        ));
        assert!(jobs.running.is_none() && jobs.pending.is_none());
        assert!(store.entries().unwrap().is_empty());
    }
}
