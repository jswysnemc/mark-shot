//! 【截图另存为】【文件任务】单个临时线程与一个可替换待处理任务，完成后唤醒原生窗口

use std::{
    io,
    path::PathBuf,
    sync::{Arc, OnceLock, mpsc},
    thread::JoinHandle,
};

use crate::{
    backend::window::WindowWake,
    ui::file_picker::{
        Listing,
        save::{self, ExistingFile},
    },
};

pub(super) type Completion = Arc<OnceLock<Result<PathBuf, String>>>;

pub(super) enum Task {
    List {
        path: PathBuf,
        hidden: bool,
        create: bool,
    },
    MakeFolder(PathBuf),
    Save {
        path: PathBuf,
        expected: Option<ExistingFile>,
        png: Arc<Vec<u8>>,
    },
}

pub(super) enum Response {
    Listed {
        requested: PathBuf,
        listing: io::Result<Listing>,
    },
    MadeFolder(io::Result<()>),
    Saved(io::Result<()>),
    Failed(String),
}

pub(super) struct Jobs {
    sender: mpsc::Sender<Response>,
    receiver: mpsc::Receiver<Response>,
    running: Option<JoinHandle<()>>,
    pending: Option<Task>,
    wake: WindowWake,
    completion: Completion,
}

impl Jobs {
    /// 【截图另存为】【调度创建】用 wake 和 completion 创建空调度器；不启动线程，不读取目录
    pub fn new(wake: WindowWake, completion: Completion) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver,
            running: None,
            pending: None,
            wake,
            completion,
        }
    }

    /// 【截图另存为】【任务提交】执行 task 或替换唯一待处理任务；同一窗口最多一个工作线程
    pub fn submit(&mut self, task: Task) {
        if self.running.is_some() {
            self.pending = Some(task);
            return;
        }
        self.start(task);
    }

    /// 【截图另存为】【线程启动】启动 task，异常转为界面错误；完成后通知窗口，不持续轮询
    fn start(&mut self, task: Task) {
        let saving = matches!(&task, Task::Save { .. });
        let sender = self.sender.clone();
        let wake = self.wake.clone();
        let completion = self.completion.clone();
        match std::thread::Builder::new()
            .name("save-as-files".into())
            .spawn(move || {
                // 1. 【截图另存为】【异常收束】无论成功或失败都交付结果，避免界面永久等待
                let response = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    execute(task, &completion)
                }))
                .unwrap_or_else(|_| {
                    if saving {
                        let _ = completion.set(Err("file worker failed".into()));
                    }
                    Response::Failed("file worker failed".into())
                });
                let _ = sender.send(response);
                wake.notify();
            }) {
            Ok(handle) => self.running = Some(handle),
            Err(error) => {
                if saving {
                    let _ = self.completion.set(Err(error.to_string()));
                }
                let _ = self.sender.send(Response::Failed(error.to_string()));
                self.wake.notify();
            }
        }
    }

    /// 【截图另存为】【结果接收】接收完成结果并回收线程，随后执行最新待处理任务；无结果时返回 None
    pub fn take(&mut self) -> Option<Response> {
        let response = self.receiver.try_recv().ok()?;
        if let Some(handle) = self.running.take() {
            let _ = handle.join();
        }
        if let Some(task) = self.pending.take() {
            self.start(task);
        }
        Some(response)
    }
}

impl Drop for Jobs {
    /// 【截图另存为】【关闭回收】丢弃尚未开始的任务并等待当前文件操作结束；不遗留后台线程
    fn drop(&mut self) {
        self.pending = None;
        if let Some(handle) = self.running.take() {
            let _ = handle.join();
        }
    }
}

/// 【截图另存为】【文件执行】执行 task；实际保存结果写入 completion，窗口关闭也不会把成功误报为取消
fn execute(task: Task, completion: &Completion) -> Response {
    match task {
        Task::List {
            path,
            hidden,
            create,
        } => {
            let listing = if create {
                std::fs::create_dir_all(&path).and_then(|()| Listing::read(&path, hidden))
            } else {
                Listing::read(&path, hidden)
            };
            Response::Listed {
                requested: path,
                listing,
            }
        }
        Task::MakeFolder(path) => Response::MadeFolder(std::fs::create_dir(path)),
        Task::Save {
            path,
            expected,
            png,
        } => {
            let result = save::write(&png, &path, expected);
            // 1. 【截图另存为】【完成事实】先保存真实结果，再通知界面，关闭窗口不能撤回已经完成的原子写入
            let outcome = result.as_ref().map(|()| path).map_err(ToString::to_string);
            let _ = completion.set(outcome);
            Response::Saved(result)
        }
    }
}
