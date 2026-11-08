//! 【识别运行库】【显式安装】只在用户点击下载时联网，识别入口仅加载已安装并校验的运行库

mod install;
pub mod manifest;
#[cfg(test)]
mod tests;

use super::models::ModelStatus;
#[cfg(feature = "ocr-dynamic")]
use std::sync::{Mutex, OnceLock};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
};

#[cfg(all(feature = "ocr-dynamic", feature = "bundled-ocr"))]
compile_error!(
    "ocr-dynamic and bundled-ocr are exclusive; GPU/offline builds require --no-default-features --features ocr-gpu or bundled-ocr"
);
#[cfg(not(any(feature = "ocr-dynamic", feature = "bundled-ocr")))]
compile_error!("select ocr-dynamic (default) or bundled-ocr");

/// 【识别运行库】【版本目录】无参数，返回当前平台固定版本的用户安装目录，目录不可用时返回错误
pub fn directory() -> Result<PathBuf, String> {
    let dist =
        manifest::current().ok_or("当前平台没有运行库发行包 / Unsupported runtime platform")?;
    Ok(dirs::data_dir()
        .ok_or("用户数据目录不可用 / No data directory")?
        .join("mark-shot")
        .join("runtime")
        .join(format!(
            "onnxruntime-{}-{}",
            manifest::VERSION,
            dist.platform
        )))
}

/// 【识别运行库】【可用检查】无参数，验证本地文件摘要，不执行库代码或下载
pub fn installed() -> bool {
    if cfg!(feature = "bundled-ocr") {
        return true;
    }
    let (Ok(root), Some(dist)) = (directory(), manifest::current()) else {
        return false;
    };
    install::verify(&root.join("installed"), dist).is_ok()
}

/// 【识别运行库】【加载入口】无参数，串行校验和初始化当前进程的运行库；未安装时返回引导信息，绝不自动下载
pub fn load() -> Result<(), String> {
    #[cfg(feature = "ocr-dynamic")]
    {
        static LOADED: OnceLock<()> = OnceLock::new();
        static LOCK: Mutex<()> = Mutex::new(());
        let _lock = LOCK
            .lock()
            .map_err(|_| "运行库初始化锁失败 / Runtime initialization failed")?;
        if LOADED.get().is_some() {
            return Ok(());
        }
        let dist =
            manifest::current().ok_or("当前平台没有运行库发行包 / Unsupported runtime platform")?;
        let root = directory()?.join("installed");
        install::verify(&root, dist).map_err(|error| {
            format!("请在 OCR 面板下载或修复推理库 / Install or repair the OCR runtime: {error}")
        })?;
        ort::init_from(root.join(dist.library.file))
            .map_err(|error| format!("Install or repair the OCR runtime: {error}"))?
            .commit();
        let _ = LOADED.set(());
    }
    Ok(())
}

enum Update {
    Progress(u8),
    Done(Result<(), String>),
}

pub struct Installer {
    pub status: ModelStatus,
    pub error: Option<String>,
    pending: Option<(Receiver<Update>, Arc<AtomicBool>)>,
}

impl Default for Installer {
    /// 【识别运行库】【安装状态】无参数，读取本地存在状态，不创建下载线程
    fn default() -> Self {
        Self {
            status: if installed() {
                ModelStatus::Installed
            } else {
                ModelStatus::Missing
            },
            error: None,
            pending: None,
        }
    }
}

impl Installer {
    /// 【识别运行库】【下载启动】无参数，仅由明确下载按钮调用，已在下载时保持原任务
    pub fn start(&mut self) {
        if self.pending.is_some() || cfg!(feature = "bundled-ocr") {
            return;
        }
        let (Ok(root), Some(dist)) = (directory(), manifest::current()) else {
            self.error = Some("当前平台没有可下载的推理库 / No runtime for this platform".into());
            self.status = ModelStatus::Failed;
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result = install::download(&root, dist, &flag, |value| {
                let _ = sender.send(Update::Progress(value));
            });
            let _ = sender.send(Update::Done(result));
        });
        self.pending = Some((receiver, cancel));
        self.status = ModelStatus::Queued;
        self.error = None;
    }

    /// 【识别运行库】【进度接收】无参数，消费后台安装结果，返回是否需要刷新界面
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        let mut finished = false;
        if let Some((receiver, cancel)) = &self.pending {
            loop {
                match receiver.try_recv() {
                    Ok(Update::Progress(value)) => {
                        self.status = ModelStatus::Downloading(value);
                        changed = true;
                    }
                    Ok(Update::Done(result)) => {
                        self.status = if result.is_ok() {
                            ModelStatus::Installed
                        } else if cancel.load(Ordering::Relaxed) {
                            ModelStatus::Missing
                        } else {
                            ModelStatus::Failed
                        };
                        self.error = if cancel.load(Ordering::Relaxed) {
                            None
                        } else {
                            result.err()
                        };
                        changed = true;
                        finished = true;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.status = ModelStatus::Failed;
                        self.error =
                            Some("运行库安装任务中断 / Runtime installer disconnected".into());
                        changed = true;
                        finished = true;
                        break;
                    }
                }
            }
        }
        if finished {
            self.pending = None;
        }
        changed
    }

    /// 【识别运行库】【取消下载】无参数，要求后台停止，保留可续传片段直到显式重试
    pub fn cancel(&mut self) {
        if let Some((_, flag)) = &self.pending {
            flag.store(true, Ordering::Relaxed);
        }
    }
    /// 【识别运行库】【轮询状态】无参数，返回是否仍有所属安装任务
    pub fn needs_poll(&self) -> bool {
        self.pending.is_some()
    }
}

impl Drop for Installer {
    /// 【识别运行库】【窗口结束】无参数，退出所属界面时取消安装，不启动下一任务
    fn drop(&mut self) {
        self.cancel();
    }
}
