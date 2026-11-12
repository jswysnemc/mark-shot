//! 【录制状态】【异步查询】截图线程只收取结果，不执行阻塞网络请求

use super::Status;
use std::{
    sync::mpsc::{Receiver, TryRecvError},
    time::{Duration, Instant},
};

#[derive(Default)]
pub(crate) struct Query {
    pending: Option<Receiver<Result<Status, String>>>,
    next: Option<Instant>,
    delivery: Option<super::command_delivery::Delivery>,
}

impl Query {
    /// 【录制状态】【控制请求】按 action 排队暂停或停止，不丢弃查询尚未返回时的用户点击
    pub(crate) fn command(&mut self, action: &'static str) {
        if self.delivery.is_none() {
            match super::command_delivery::Delivery::new() {
                Ok(delivery) => self.delivery = Some(delivery),
                Err(error) => {
                    log::error!("【录制状态】【控制线程】{error}");
                    return;
                }
            }
        }
        if let Some(delivery) = &self.delivery {
            delivery.send(action);
        }
    }

    /// 【录制状态】【控制结果】无参数，非阻塞领取命令结果，已隐藏的起始界面也保留交付线程
    pub(crate) fn control_result(&self) -> Option<Result<Status, String>> {
        self.delivery.as_ref().and_then(|delivery| delivery.poll())
    }
    /// 【录制状态】【提交请求】按 action 启动一个有超时限制的后台请求，已有请求时返回 false
    pub(crate) fn request(&mut self, action: &'static str) -> bool {
        if self.pending.is_some() {
            return false;
        }
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        match std::thread::Builder::new()
            .name("capture-recording-status".into())
            .spawn(move || {
                let result = super::ipc::request(action).map_err(|error| error.to_string());
                let _ = sender.send(result);
            }) {
            Ok(_) => {
                self.pending = Some(receiver);
                self.next = None;
                true
            }
            Err(error) => {
                log::error!("【录制状态】【查询线程】{error}");
                self.next = Some(Instant::now() + Duration::from_secs(2));
                false
            }
        }
    }

    /// 【录制状态】【领取结果】按当前时刻取回结果并安排下次查询；返回空值表示无需更新
    pub(crate) fn poll(&mut self) -> Option<Result<Status, String>> {
        if let Some(result) = self.control_result() {
            return Some(result);
        }
        if let Some(receiver) = &self.pending {
            let result = match receiver.try_recv() {
                Ok(value) => value,
                Err(TryRecvError::Empty) => return None,
                Err(TryRecvError::Disconnected) => Err("Recording status unavailable".into()),
            };
            self.pending = None;
            self.next = Some(Instant::now() + Duration::from_secs(1));
            return Some(result);
        }
        if self.next.is_none_or(|next| next <= Instant::now()) {
            self.request("status");
        }
        None
    }

    /// 【录制状态】【唤醒间隔】无参数，返回领取结果或下次请求所需的最短等待时间
    pub(crate) fn timeout(&self) -> Duration {
        if self.pending.is_some() {
            Duration::from_millis(50)
        } else {
            self.next.map_or(Duration::ZERO, |next| {
                next.saturating_duration_since(Instant::now())
            })
        }
    }
}
