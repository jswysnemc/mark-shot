//! 【翻译任务】【按需生命周期】每次任务独立取消，完成只通知一次，静止窗口不持有运行时线程

use super::{
    Error, ErrorKind, Output, Provider, Result, http::Http, process, providers, request::Request,
    resolve,
};
use crate::{backend::window::WindowWake, config::Translation};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::Duration,
};
use tokio::sync::oneshot;

pub struct Task {
    pub id: u64,
    pub provider: Provider,
    cancel: Option<oneshot::Sender<()>>,
    result: Option<mpsc::Receiver<Result<Output>>>,
    worker: Option<JoinHandle<()>>,
}

impl Drop for Task {
    /// 【翻译任务】【所属回收】取消当前所属任务，无参数、无返回值；只连接已结束线程，不阻塞窗口等待网络
    fn drop(&mut self) {
        self.cancel();
        if self.worker.as_ref().is_some_and(JoinHandle::is_finished) {
            let _ = self.worker.take().unwrap().join();
        }
    }
}

impl Task {
    /// 【翻译任务】【退出状态】返回所属工作线程是否已经结束，无参数，不以结果通知代替线程真实退出
    pub(crate) fn is_finished(&self) -> bool {
        self.worker.as_ref().is_none_or(JoinHandle::is_finished)
    }

    /// 【翻译任务】【窗口关闭回收】取消并等待所属线程真实退出，无参数、无返回值，只用于窗口事件循环退出后，不用于交互取消
    pub(crate) fn shutdown(&mut self) {
        self.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }

    /// 【翻译任务】【真实开始】按 config、source、target、config_path 和 wake 启动一次任务，返回身份；只在主动翻译时读取凭据环境
    pub fn start(
        config: &Translation,
        source: &str,
        target: &str,
        config_path: &Path,
        wake: WindowWake,
    ) -> Result<Self> {
        Self::start_with(config, source, target, config_path, wake, &|name| {
            std::env::var(name).ok()
        })
    }

    /// 【翻译任务】【注入开始】按配置、确认文本、路径、通知和 env 解析并启动，返回任务；测试不需要读取用户凭据或连接图形会话
    fn start_with(
        config: &Translation,
        source: &str,
        target: &str,
        config_path: &Path,
        wake: WindowWake,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Self> {
        if source.len() > 8 * 1024 * 1024 {
            return Err(Error::new(
                ErrorKind::Configuration,
                "translation source exceeds eight MiB",
            ));
        }
        Self::start_request_with(
            config,
            Request::text(source),
            target,
            config_path,
            move || wake.notify(),
            env,
        )
    }

    /// 【翻译任务】【原图开始】按 config、request、target、config_path 和 wake 启动带真实坐标的请求，返回一次任务，通知可接入截图或贴图事件循环
    pub fn start_request(
        config: &Translation,
        request: Request,
        target: &str,
        config_path: &Path,
        wake: impl FnOnce() + Send + 'static,
    ) -> Result<Self> {
        Self::start_request_with(config, request, target, config_path, wake, &|name| {
            std::env::var(name).ok()
        })
    }

    /// 【翻译任务】【共用开始】按请求快照及 env 解析稳定服务并启动，wake 在结果发送后调用一次，返回任务或启动错误
    fn start_request_with(
        config: &Translation,
        request: Request,
        target: &str,
        config_path: &Path,
        wake: impl FnOnce() + Send + 'static,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Self> {
        request.validate(target)?;
        if config.provider.trim().eq_ignore_ascii_case("command")
            && config.command.trim().is_empty()
        {
            return Err(Error::new(
                ErrorKind::Configuration,
                "请先填写翻译命令 / Translation command is required",
            ));
        }
        let segments = request.segments();
        // 1. 【翻译任务】【稳定选择】命令优先于服务偏好，已经选择的服务不会因运行失败把原文交给其他厂商
        let service = if config.command.trim().is_empty()
            && !config.provider.trim().eq_ignore_ascii_case("helper")
        {
            Some(resolve::service(&config.root(), &config.provider, env)?)
        } else {
            None
        };
        let provider = service.as_ref().map_or(
            if config.command.trim().is_empty() {
                Provider::Helper
            } else {
                Provider::Command
            },
            resolve::Service::provider,
        );
        let (cancel, cancellation) = oneshot::channel();
        let (send, result) = mpsc::channel();
        let timeout = match &service {
            Some(resolve::Service::Llm(service)) => service.timeout,
            Some(resolve::Service::Tencent(service)) => service.timeout,
            Some(resolve::Service::Form(service)) => service.timeout,
            None => Duration::from_millis(config.timeout_ms.clamp(1000, 300000)),
        };
        let target = target.to_owned();
        let template = config.command.clone();
        let config_path: PathBuf = config_path.to_owned();
        let worker = std::thread::Builder::new().name("mark-shot-translate".into()).spawn(move || {
            // 2. 【翻译任务】【单次运行时】运行时只属于本次工作线程，完成、超时或取消后回收所有连接
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()
                    .map_err(|_| Error::new(ErrorKind::StartFailed, "cannot initialize translation runtime"))?;
                let result = runtime.block_on(async {
                    if let Some(service) = &service {
                        let work = async { providers::translate(service, &segments, &target, &Http::new()?).await };
                        tokio::select! {
                            biased;
                            _ = cancellation => Err(Error::new(ErrorKind::Canceled, "Translation canceled")),
                            result = tokio::time::timeout(timeout, work) => result.unwrap_or_else(|_| Err(Error::new(ErrorKind::Timeout, "timed out"))),
                        }
                    } else {
                        process::run_request(provider, &template, &request, &target, &config_path, cancellation, timeout).await
                    }
                });
                runtime.shutdown_timeout(Duration::from_millis(100));
                result
            })).unwrap_or_else(|_| Err(Error::new(ErrorKind::StartFailed, "translation worker failed")));
            // 3. 【翻译任务】【一次交付】接收方已经销毁时丢弃旧结果，通知不会创建周期工作或更改新任务身份
            let result = result.map_err(|mut error| { error.provider = Some(provider); error });
            let _ = send.send(result);
            wake();
        }).map_err(|_| Error::new(ErrorKind::StartFailed, "cannot start translation worker"))?;
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Ok(Self {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            provider,
            cancel: Some(cancel),
            result: Some(result),
            worker: Some(worker),
        })
    }

    /// 【翻译任务】【主动取消】发送一次取消通知，无参数、无返回值，不阻塞窗口等待读取结束
    pub fn cancel(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
    }

    /// 【翻译任务】【完成读取】返回当前任务一次性结果，无参数，未结束返回空值；不会重复交付或覆盖调用方原文
    pub fn take_result(&mut self) -> Option<Result<Output>> {
        let receiver = self.result.as_ref()?;
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => Err(Error::new(
                ErrorKind::StartFailed,
                "translation worker disconnected",
            )),
        };
        self.result = None;
        self.cancel = None;
        Some(result)
    }
}

#[cfg(test)]
mod tests;
