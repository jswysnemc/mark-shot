//! 【Windows控件识别】【后台线程】冻结窗口决定命中归属，UIA只在独立线程探测对应窗口

use super::automation::Automation;
use crate::{
    backend::{
        task_wake::Wake,
        window_detection::{
            Window,
            requests::{Hit, Queue},
        },
    },
    features::region_detection::PixelRect,
};
use std::sync::Arc;

pub(crate) struct Worker(Arc<Queue>);
impl Worker {
    /// 【Windows控件识别】【线程启动】按windows和wake创建会话工作线程，返回无等待请求句柄
    pub fn new(windows: Vec<Window>, wake: Wake) -> std::io::Result<Self> {
        let shared = Arc::new(Queue::default());
        let queue = shared.clone();
        std::thread::Builder::new()
            .name("window-controls".into())
            .spawn(move || {
                let mut automation = match Automation::new() {
                    Ok(automation) => Some(automation),
                    Err(error) => {
                        log::info!("【Windows控件识别】【服务不可用】{error}");
                        None
                    }
                };
                while let Some(request) = queue.next() {
                    let window = windows.iter().find(|window| {
                        window.native_id != 0
                            && request.point.0 as f32 >= window.rect.left()
                            && (request.point.0 as f32) < window.rect.right()
                            && request.point.1 as f32 >= window.rect.top()
                            && (request.point.1 as f32) < window.rect.bottom()
                    });
                    let Some(window) = window else {
                        if queue.publish(request, Vec::new()) {
                            wake.notify();
                        }
                        continue;
                    };
                    let bounds = PixelRect::new(
                        window.rect.left() as i32,
                        window.rect.top() as i32,
                        window.rect.width() as i32,
                        window.rect.height() as i32,
                    );
                    if let Some(automation) = &mut automation {
                        for _attempt in 0..3 {
                            let complete = automation.query(
                                window.native_id,
                                bounds,
                                request.point,
                                || queue.cancelled(request),
                                |path| {
                                    if queue.publish(request, path) {
                                        wake.notify();
                                    }
                                },
                            );
                            if complete || queue.cancelled(request) {
                                break;
                            }
                        }
                    } else if queue.publish(request, vec![bounds]) {
                        wake.notify();
                    }
                }
            })?;
        Ok(Self(shared))
    }
    /// 【Windows控件识别】【移动交付】按point合并最新位置，无返回值，不在界面线程调用系统控件
    pub fn request(&self, point: (i32, i32)) {
        self.0.request(point);
    }
    /// 【Windows控件识别】【完成接收】无参数，取走最新控件路径，不等待后台
    pub fn take(&self) -> Option<Hit> {
        self.0.take()
    }
}
impl Drop for Worker {
    /// 【Windows控件识别】【快速退出】无参数，通知线程取消并自行释放COM，截图窗口关闭不等待外部服务
    fn drop(&mut self) {
        self.0.stop();
    }
}
