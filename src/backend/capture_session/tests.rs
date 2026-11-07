//! 【持续捕获测试】【生命周期】候选只在进入时探测，验证首帧保留及失败后的统一关闭

use super::*;
use crate::backend::capture_chain::{Candidate, CaptureChain};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct Method {
    counts: Arc<[AtomicUsize; 3]>,
    succeeds: bool,
}
struct Session {
    counts: Arc<[AtomicUsize; 3]>,
    succeeds: bool,
    closed: bool,
}

#[async_trait]
impl CaptureMethod for Method {
    /// 【持续捕获测试】【禁止旧入口】持续会话不得调用普通单次捕获，无参数，返回明确错误
    async fn capture_frame(&self, _: &[Output]) -> crate::AppResult<CaptureResult> {
        Err("single-shot path must not be used".into())
    }
    /// 【持续捕获测试】【创建计数】记录唯一会话创建，outputs仅代表测试绑定，返回可验证会话
    async fn open_session(&self, _: &[Output]) -> Result<Box<dyn CaptureSession>> {
        self.counts[0].fetch_add(1, Ordering::Relaxed);
        Ok(Box::new(Session {
            counts: self.counts.clone(),
            succeeds: self.succeeds,
            closed: false,
        }))
    }
}

#[async_trait]
impl CaptureSession for Session {
    /// 【持续捕获测试】【帧计数】每次返回不同帧编号或失败，无参数，禁止已关闭会话继续读取
    async fn next_frame(&mut self) -> Result<CaptureResult> {
        if self.closed {
            return Err("closed".into());
        }
        let sequence = self.counts[1].fetch_add(1, Ordering::Relaxed) + 1;
        if !self.succeeds {
            return Err("native frame failed".into());
        }
        Ok(CaptureResult {
            frames: vec![crate::types::MonitorFrame {
                output: 0,
                pixels: vec![sequence as u8, 0, 0, 255],
                pw_width: 1,
                pw_height: 1,
                pw_stride: 4,
                info: crate::types::StreamInfo {
                    node_id: 1,
                    size: Some((1, 1)),
                    position: Some((0, 0)),
                },
            }],
        })
    }
    /// 【持续捕获测试】【关闭计数】首次关闭释放会话，重复调用保持相同计数，无参数
    async fn close(&mut self) -> Result<()> {
        if !self.closed {
            self.closed = true;
            self.counts[2].fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }
}

/// 【持续捕获测试】【探测复用】失败候选关闭一次，成功候选只创建一次且首帧不重采集，后续读取不反复探测，无参数
#[tokio::test]
async fn first_frame_fallback_reuses_the_selected_session_and_closes_every_candidate() {
    let first = Arc::new([
        AtomicUsize::new(0),
        AtomicUsize::new(0),
        AtomicUsize::new(0),
    ]);
    let second = Arc::new([
        AtomicUsize::new(0),
        AtomicUsize::new(0),
        AtomicUsize::new(0),
    ]);
    let chain = CaptureChain::new(vec![
        Candidate {
            name: "native",
            method: Box::new(Method {
                counts: first.clone(),
                succeeds: false,
            }),
        },
        Candidate {
            name: "portal",
            method: Box::new(Method {
                counts: second.clone(),
                succeeds: true,
            }),
        },
    ]);
    let mut session = chain.open_session(&[]).await.unwrap();
    for sequence in 1..=4 {
        assert_eq!(
            session.next_frame().await.unwrap().frames[0].pixels[0],
            sequence
        );
    }
    session.close().await.unwrap();
    session.close().await.unwrap();
    assert!(session.next_frame().await.is_err());
    assert_eq!(
        first
            .iter()
            .map(|value| value.load(Ordering::Relaxed))
            .collect::<Vec<_>>(),
        [1, 1, 1]
    );
    assert_eq!(
        second
            .iter()
            .map(|value| value.load(Ordering::Relaxed))
            .collect::<Vec<_>>(),
        [1, 4, 1]
    );
}
