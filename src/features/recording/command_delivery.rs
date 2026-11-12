//! 【录制控制】【独立交付】用户控制命令按顺序发送，起始界面关闭仍完成已接收的请求

use super::Status;
use std::sync::mpsc::{Receiver, Sender};

type Reply = Result<Status, String>;

pub(super) struct Delivery {
    sender: Option<Sender<&'static str>>,
    replies: Receiver<Reply>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Delivery {
    /// 【录制控制】【交付线程】按 request 创建顺序执行请求的线程，返回创建错误，不依赖界面轮询
    fn start(request: impl Fn(&'static str) -> Reply + Send + 'static) -> std::io::Result<Self> {
        let (sender, incoming) = std::sync::mpsc::channel();
        let (reply, replies) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("capture-recording-control".into())
            .spawn(move || {
                for action in incoming {
                    let result = request(action);
                    if let Err(error) = &result {
                        log::error!("【录制控制】【命令交付】{error}");
                    }
                    let _ = reply.send(result);
                }
            })?;
        Ok(Self {
            sender: Some(sender),
            replies,
            thread: Some(thread),
        })
    }

    /// 【录制控制】【本机会话】无参数，返回使用现有本机 IPC 的独立命令交付器
    pub(super) fn new() -> std::io::Result<Self> {
        Self::start(|action| super::ipc::request(action).map_err(|error| error.to_string()))
    }

    /// 【录制控制】【发送意图】按 action 立即排队，返回是否成功；命令不依赖状态查询结果
    pub(super) fn send(&self, action: &'static str) -> bool {
        self.sender
            .as_ref()
            .is_some_and(|sender| sender.send(action).is_ok())
    }

    /// 【录制控制】【交付结果】无参数，返回已完成命令的最新状态或错误，无结果时不阻塞
    pub(super) fn poll(&self) -> Option<Reply> {
        self.replies.try_recv().ok()
    }
}

impl Drop for Delivery {
    /// 【录制控制】【退出收尾】无参数，关闭队列后等待已接收命令发出，防止退出进程丢失停止请求
    fn drop(&mut self) {
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【录制控制测试】【界面关闭】命令尚未交付时销毁入口，仍按原顺序发送暂停和停止，不相互覆盖
    #[test]
    fn closing_intro_drains_commands_in_order() {
        let observed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let result = observed.clone();
        let delivery = Delivery::start(move |action| {
            result.lock().unwrap().push(action);
            Ok(Status::default())
        })
        .unwrap();
        assert!(delivery.send("pause"));
        assert!(delivery.send("stop"));
        drop(delivery);
        assert_eq!(*observed.lock().unwrap(), ["pause", "stop"]);
    }
}
