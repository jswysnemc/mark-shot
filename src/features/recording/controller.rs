//! 【录制控制】【线程生命周期】控制消息有界，状态合并，停止完成后才发布文件

use super::{Command, Status, Target, backend, output::Output};
use std::sync::{
    Arc, Mutex,
    mpsc::{self, SyncSender},
};

pub struct Controller {
    pub status: backend::Shared,
    pub sender: SyncSender<Command>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Controller {
    /// 【录制控制】【启动工作线程】按 config 与 target 启动一次会话，返回控制器或配置错误
    pub fn start(
        config: crate::config::recording::Recording,
        target: Target,
    ) -> Result<Self, String> {
        config.validate()?;
        let output = Output::create(&config)?;
        let (sender, commands) = mpsc::sync_channel(8);
        let status = Arc::new(Mutex::new(Status {
            phase: "countdown".into(),
            output: Some(output.destination.to_string_lossy().into()),
            ..Default::default()
        }));
        let shared = status.clone();
        let worker = std::thread::Builder::new()
            .name("recording".into())
            .spawn(move || {
                // 1. 【录制控制】【倒计时取消】阻塞等待可取消的期限，不创建采集设备
                let until = std::time::Instant::now()
                    + std::time::Duration::from_secs(u64::from(config.countdown));
                while let Some(remaining) = until.checked_duration_since(std::time::Instant::now())
                {
                    backend::publish(
                        &shared,
                        "countdown",
                        &remaining.as_secs().saturating_add(1).to_string(),
                        0,
                    );
                    match commands
                        .recv_timeout(remaining.min(std::time::Duration::from_millis(200)))
                    {
                        Ok(Command::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                            let _ = std::fs::remove_dir_all(&output.directory);
                            backend::publish(&shared, "cancelled", "", 0);
                            return;
                        }
                        _ => {}
                    }
                }
                // 2. 【录制控制】【平台会话】设备与编码器资源只由后端持有，错误保留工作文件
                backend::publish(&shared, "starting", "", 0);
                let result = backend::run(&config, &target, &output.video, &commands, &shared)
                    .and_then(|()| {
                        let elapsed = shared.lock().unwrap().elapsed_ms;
                        backend::publish(&shared, "saving", "", elapsed);
                        super::export::finish(&config, &output, &commands, &shared)
                    });
                let elapsed = shared.lock().unwrap().elapsed_ms;
                match result {
                    Ok(()) => backend::publish(
                        &shared,
                        "finished",
                        &output.destination.to_string_lossy(),
                        elapsed,
                    ),
                    Err(error) => {
                        let detail = format!(
                            "{error}\n恢复目录 / Recovery: {}",
                            output.directory.display()
                        );
                        log::error!("【屏幕录制】【会话失败】{detail}");
                        backend::publish(&shared, "failed", &detail, elapsed);
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            status,
            sender,
            worker: Some(worker),
        })
    }

    /// 【录制控制】【状态快照】无参数，返回最新状态，不保留视频帧或历史通知
    pub fn snapshot(&self) -> Status {
        self.status
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    /// 【录制控制】【命令投递】按 command 投递有界控制消息，返回队列错误；运行时命令统一经由本地命令服务投递
    #[cfg(test)]
    pub fn send(&self, command: Command) -> Result<(), String> {
        self.sender
            .try_send(command)
            .map_err(|error| error.to_string())
    }

    /// 【录制控制】【结束判断】无参数，返回工作线程是否已结束
    pub fn finished(&self) -> bool {
        self.worker
            .as_ref()
            .is_none_or(|worker| worker.is_finished())
    }
}

impl Drop for Controller {
    /// 【录制控制】【资源清理】无参数，要求停止并等待后端收尾，避免退出后残留录制
    fn drop(&mut self) {
        let _ = self.sender.send(Command::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;
