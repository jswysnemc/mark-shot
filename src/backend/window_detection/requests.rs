//! 【控件识别】【最新请求】只保存一个待办位置和一个结果，指针移动立即使旧探测失效

use crate::features::region_detection::PixelRect;
use std::sync::{
    Condvar, Mutex,
    atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Copy)]
pub(crate) struct Request {
    pub revision: u64,
    pub point: (i32, i32),
}
pub(crate) struct Hit {
    pub point: (i32, i32),
    pub path: Vec<PixelRect>,
}
#[derive(Default)]
struct Pending {
    request: Option<Request>,
    result: Option<Hit>,
    stopped: bool,
}
#[derive(Default)]
pub(crate) struct Queue {
    pending: Mutex<Pending>,
    changed: Condvar,
    revision: AtomicU64,
}

impl Queue {
    /// 【控件识别】【合并位置】按point替换尚未处理的位置，无返回值，连续移动不创建任务队列
    pub fn request(&self, point: (i32, i32)) {
        let mut state = self.pending.lock().unwrap();
        let revision = self.revision.fetch_add(1, Ordering::Relaxed) + 1;
        state.request = Some(Request { revision, point });
        state.result = None;
        self.changed.notify_one();
    }
    /// 【控件识别】【等待请求】无参数，阻塞到最新请求或关闭，返回待处理请求，不消耗空闲CPU
    pub fn next(&self) -> Option<Request> {
        let mut state = self.pending.lock().unwrap();
        while state.request.is_none() && !state.stopped {
            state = self.changed.wait(state).unwrap();
        }
        if state.stopped {
            None
        } else {
            state.request.take()
        }
    }
    /// 【控件识别】【代次检查】按request检查是否被新位置或关闭替代，返回是否取消
    pub fn cancelled(&self, request: Request) -> bool {
        self.revision.load(Ordering::Relaxed) != request.revision
    }
    /// 【控件识别】【结果发布】按request与path替换本次结果，只接受最新代次，返回是否需要唤醒界面
    pub fn publish(&self, request: Request, path: Vec<PixelRect>) -> bool {
        let mut state = self.pending.lock().unwrap();
        if state.stopped || self.cancelled(request) {
            return false;
        }
        state.result = Some(Hit {
            point: request.point,
            path,
        });
        true
    }
    /// 【控件识别】【结果接收】无参数，取走最新完整路径或返回空值，不等待服务进程
    pub fn take(&self) -> Option<Hit> {
        self.pending.lock().unwrap().result.take()
    }
    /// 【控件识别】【会话结束】无参数，立即取消所有请求并唤醒退出，不等待慢控件调用
    pub fn stop(&self) {
        let mut state = self.pending.lock().unwrap();
        state.stopped = true;
        state.request = None;
        state.result = None;
        self.revision.fetch_add(1, Ordering::Relaxed);
        self.changed.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【控件识别测试】【过期丢弃】无参数，模拟慢服务与快速移动，验证只处理最新位置且拒绝晚到结果
    #[test]
    fn rapid_motion_replaces_pending_work_and_rejects_stale_results() {
        let queue = Queue::default();
        queue.request((-100, 40));
        let first = queue.next().unwrap();
        for offset in 0..1000 {
            queue.request((offset, 50));
        }
        assert!(queue.cancelled(first));
        assert!(!queue.publish(first, vec![]));
        let last = queue.next().unwrap();
        assert_eq!(last.point, (999, 50));
        let rect = PixelRect::new(990, 30, 40, 40);
        assert!(queue.publish(last, vec![rect]));
        let result = queue.take().unwrap();
        assert_eq!(result.point, last.point);
        assert_eq!(result.path, vec![rect]);
        queue.stop();
        assert!(queue.cancelled(last));
        assert!(queue.next().is_none());
        assert!(!queue.publish(last, vec![rect]));
    }
}
