//! 【Windows控件识别】【后台调度】首次与停留查询分别持有COM，慢补充查询不会占用新指针的处理线程

use super::automation::Automation;
use crate::{
    backend::{
        task_wake::Wake,
        window_detection::{
            Window,
            requests::{Hit, Queue},
            uia::{Policy, Stop},
        },
    },
    features::region_detection::PixelRect,
};
use std::{sync::Arc, time::Duration};

pub(crate) struct Worker(Arc<Queue>);
impl Worker {
    /// 【Windows控件识别】【线程启动】按windows与wake创建独立首次及补充线程，返回无等待句柄，启动失败时撤销已有线程
    pub fn new(windows: Vec<Window>, wake: Wake) -> std::io::Result<Self> {
        let shared = Arc::new(Queue::default());
        for refining in [false, true] {
            let queue = shared.clone();
            let windows = windows.clone();
            let wake = wake.clone();
            if let Err(error) = std::thread::Builder::new()
                .name(
                    if refining {
                        "window-controls-refine"
                    } else {
                        "window-controls"
                    }
                    .into(),
                )
                .spawn(move || run(windows, queue, wake, refining))
            {
                shared.stop();
                return Err(error);
            }
        }
        Ok(Self(shared))
    }
    /// 【Windows控件识别】【移动交付】按point合并最新位置，无返回值，不在界面线程调用系统控件
    pub fn request(&self, point: (i32, i32)) {
        self.0.request(point);
    }
    /// 【Windows控件识别】【整窗取消】无参数，撤销组件探测与补充预约，不销毁会话线程
    pub fn cancel(&self) {
        self.0.cancel();
    }
    /// 【Windows控件识别】【完成接收】无参数，取走最新控件路径，不等待后台
    pub fn take(&self) -> Option<Hit> {
        self.0.take()
    }
}
impl Drop for Worker {
    /// 【Windows控件识别】【快速退出】无参数，取消两个线程并自行释放COM，关闭截图不等待外部服务
    fn drop(&mut self) {
        self.0.stop();
    }
}

/// 【Windows控件识别】【查询线程】按windows、queue、wake与refining处理对应阶段，无返回值，外部接口只在本线程调用
fn run(windows: Vec<Window>, queue: Arc<Queue>, wake: Wake, refining: bool) {
    let mut automation = match Automation::new() {
        Ok(automation) => Some(automation),
        Err(error) => {
            log::info!("【Windows控件识别】【服务不可用】{error}");
            None
        }
    };
    loop {
        let request = if refining {
            queue.next_refinement(Duration::from_millis(80))
        } else {
            queue.next()
        };
        let Some(request) = request else { break };
        let window = windows.iter().find(|window| {
            window.native_id != 0
                && request.point.0 as f32 >= window.rect.left()
                && (request.point.0 as f32) < window.rect.right()
                && request.point.1 as f32 >= window.rect.top()
                && (request.point.1 as f32) < window.rect.bottom()
        });
        let Some(window) = window else {
            if !refining && queue.publish(request, Vec::new()) {
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
        let mut publish = |path| {
            let published = if refining {
                queue.publish_refinement(request, path)
            } else {
                queue.publish(request, path)
            };
            if published {
                wake.notify();
            }
        };
        // 1. 【Windows控件识别】【可靠回退】每次查询都有窗口范围，失败和超时不会留下上一位置的候选
        let stop = if let Some(automation) = &mut automation {
            let result = automation.query_with_policy(
                window.native_id,
                bounds,
                request.point,
                if refining {
                    Policy::refinement()
                } else {
                    Policy::foreground()
                },
                || queue.cancelled(request),
                &mut publish,
            );
            if result.stop != Stop::Cancelled {
                publish(result.path);
            }
            result.stop
        } else {
            publish(vec![bounds]);
            Stop::Failure
        };
        // 2. 【Windows控件识别】【停留补充】完整缓存路径不安排补充，未完成路径只预约当前代次
        if !refining && stop.refinable() {
            queue.refine(request);
        }
    }
}
