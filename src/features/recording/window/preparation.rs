//! 【录制准备】【状态交接】倒计时小组件与本地命令服务共享状态，工作线程创建前也能取消录制

use crate::{
    config::recording::Recording,
    features::recording::{Command, Status, backend},
};
use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, SyncSender},
};

pub(super) struct Preparation {
    pub status: backend::Shared,
    pub sender: SyncSender<Command>,
    pub commands: Receiver<Command>,
}

impl Preparation {
    /// 【录制准备】【状态创建】按 config 创建有界命令通道与准备状态，不创建输出文件或采集设备
    pub fn new(config: &Recording) -> Self {
        let (sender, commands) = mpsc::sync_channel(8);
        Self {
            status: Arc::new(Mutex::new(Status {
                phase: if config.countdown > 0 {
                    "countdown"
                } else {
                    "starting"
                }
                .into(),
                detail: config.countdown.to_string(),
                format: config.format.clone(),
                ..Default::default()
            })),
            sender,
            commands,
        }
    }

    /// 【录制准备】【倒数更新】按 seconds 发布小组件实际显示的剩余秒数，返回是否收到取消请求
    pub fn update(&mut self, seconds: u64) -> bool {
        let cancelled = self
            .commands
            .try_iter()
            .any(|command| matches!(command, Command::Stop));
        let detail = if cancelled {
            String::new()
        } else {
            seconds.to_string()
        };
        backend::publish(
            &self.status,
            if cancelled { "cancelled" } else { "countdown" },
            &detail,
            0,
        );
        cancelled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【录制准备测试】【外部取消】倒计时尚未创建控制器时仍发布格式和剩余秒数，停止命令取消准备阶段
    #[test]
    fn countdown_reports_its_format_and_accepts_stop_before_capture_exists() {
        let mut preparation = Preparation::new(&Recording {
            format: "gif".into(),
            countdown: 5,
            ..Default::default()
        });
        assert!(!preparation.update(3));
        let status = preparation.status.lock().unwrap().clone();
        assert_eq!(
            (
                status.phase.as_str(),
                status.detail.as_str(),
                status.format.as_str()
            ),
            ("countdown", "3", "gif")
        );
        assert!(status.output.is_none());
        preparation.sender.try_send(Command::Stop).unwrap();
        assert!(preparation.update(2));
        assert_eq!(preparation.status.lock().unwrap().phase, "cancelled");
    }
}
