//! 【滚动截图任务测试】【等待取消】慢来源不得阻塞界面事务，也不能把过期帧送入当前会话

use super::*;
use async_trait::async_trait;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

struct Session {
    requests: Arc<AtomicUsize>,
    active: Arc<AtomicUsize>,
}
struct Active(Arc<AtomicUsize>);

impl Drop for Active {
    /// 【滚动截图任务测试】【等待释放】记录异步帧等待结束或取消，无参数、无返回值
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

#[async_trait]
impl CaptureSession for Session {
    /// 【滚动截图任务测试】【慢来源】首次请求等待五秒，其后返回一次空帧，作用是观察取消和过期结果，无参数
    async fn next_frame(&mut self) -> Result<CaptureResult> {
        self.active.fetch_add(1, Ordering::Relaxed);
        let _active = Active(self.active.clone());
        let number = self.requests.fetch_add(1, Ordering::Relaxed);
        if number == 0 {
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
        Ok(CaptureResult { frames: vec![] })
    }
}

/// 【滚动截图任务测试】【快速取消】取消五秒帧等待后立即接受新请求，后台仍是同一会话，关闭不等待旧期限，无参数
#[tokio::test]
async fn slow_frame_cancellation_and_resume_do_not_wait_for_the_original_timeout() {
    let wake = crate::backend::task_wake::Source::new().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let active = Arc::new(AtomicUsize::new(0));
    let mut task = Task::new(
        Box::new(Session {
            requests: requests.clone(),
            active: active.clone(),
        }),
        wake.wake(),
    )
    .unwrap();
    task.request().unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    while requests.load(Ordering::Relaxed) == 0 {
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    let start = Instant::now();
    task.cancel();
    task.request().unwrap();
    while task.take().is_none() {
        assert!(start.elapsed() < Duration::from_millis(250));
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    let _session = task.finish().unwrap();
    assert_eq!(active.load(Ordering::Relaxed), 0);
    assert_eq!(requests.load(Ordering::Relaxed), 2);
    assert!(start.elapsed() < Duration::from_millis(250));
}

/// 【滚动截图任务测试】【结束等待】直接结束慢帧任务必须回收原会话，不等五秒请求超时，无参数
#[tokio::test]
async fn finishing_an_in_flight_frame_releases_its_waiter() {
    let wake = crate::backend::task_wake::Source::new().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let active = Arc::new(AtomicUsize::new(0));
    let mut task = Task::new(
        Box::new(Session {
            requests: requests.clone(),
            active: active.clone(),
        }),
        wake.wake(),
    )
    .unwrap();
    task.request().unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    while requests.load(Ordering::Relaxed) == 0 {
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    let start = Instant::now();
    let _session = task.finish().unwrap();
    assert!(start.elapsed() < Duration::from_millis(250));
    assert_eq!(active.load(Ordering::Relaxed), 0);
}
