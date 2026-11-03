use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;

use super::CaptureMethod;
use crate::types::{Capture, CaptureResult, Output};

pub(crate) struct Candidate {
    pub name: &'static str,
    pub method: Box<dyn CaptureMethod>,
}

pub(crate) struct CaptureChain {
    candidates: Vec<Candidate>,
    selected: AtomicUsize,
}

impl CaptureChain {
    /// 【Wayland】【捕获回退】组合有序后端；返回记忆最近成功后端的捕获器，避免长截图反复重试失败协议
    pub fn new(candidates: Vec<Candidate>) -> Self {
        Self {
            candidates,
            selected: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl CaptureMethod for CaptureChain {
    /// 【Wayland】【帧捕获】依次尝试可用后端；成功后返回像素，全部失败时返回完整诊断
    async fn capture_frame(
        &self,
        outputs: &[Output],
    ) -> Result<CaptureResult, Box<dyn std::error::Error>> {
        let selected = self
            .selected
            .load(Ordering::Relaxed)
            .min(self.candidates.len());
        let mut failures = Vec::new();
        for index in (selected..self.candidates.len()).chain(0..selected) {
            let candidate = &self.candidates[index];
            let error = match candidate.method.capture_frame(outputs).await {
                Ok(frame) if !frame.frames.is_empty() => {
                    self.selected.store(index, Ordering::Relaxed);
                    return Ok(frame);
                }
                Ok(_) => "the backend returned no frames".to_owned(),
                Err(error) => error.to_string(),
            };
            log::info!("【Wayland】【捕获回退】{} 不可用: {error}", candidate.name);
            failures.push(format!("{}: {error}", candidate.name));
        }
        Err(format!("no capture backend succeeded: {}", failures.join("; ")).into())
    }

    /// 【Wayland】【捕获准备】提前准备当前后端；准备失败不阻止后续捕获尝试其他协议
    async fn prepare(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(candidate) = self.candidates.get(self.selected.load(Ordering::Relaxed))
            && let Err(error) = candidate.method.prepare().await
        {
            log::info!(
                "【Wayland】【捕获准备】{} 暂不可用: {error}",
                candidate.name
            );
        }
        Ok(())
    }

    /// 【Wayland】【窗口捕获】寻找支持活动窗口的后端；不把显示器截图冒充窗口截图
    async fn capture_active_window(&self) -> Result<Capture, Box<dyn std::error::Error>> {
        let mut failures = Vec::new();
        for candidate in &self.candidates {
            match candidate.method.capture_active_window().await {
                Ok(capture) => return Ok(capture),
                Err(error) => failures.push(format!("{}: {error}", candidate.name)),
            }
        }
        Err(format!(
            "active-window capture is unavailable: {}",
            failures.join("; ")
        )
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MonitorFrame, StreamInfo};
    use std::sync::Arc;

    struct Mock {
        success: bool,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl CaptureMethod for Mock {
        /// 【捕获测试】【后端模拟】记录调用次数并返回固定像素或错误，避免访问真实桌面
        async fn capture_frame(
            &self,
            _: &[Output],
        ) -> Result<CaptureResult, Box<dyn std::error::Error>> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            if !self.success {
                return Err("unavailable".into());
            }
            Ok(CaptureResult {
                frames: vec![MonitorFrame {
                    output: 0,
                    pixels: vec![0, 0, 0, 255],
                    pw_width: 1,
                    pw_height: 1,
                    pw_stride: 4,
                    info: StreamInfo {
                        node_id: 0,
                        size: Some((1, 1)),
                        position: Some((0, 0)),
                    },
                }],
            })
        }
    }

    /// 【捕获测试】【成功缓存】验证失败协议只尝试一次，后续帧直接复用成功后端
    #[tokio::test]
    async fn fallback_is_reused_on_subsequent_frames() {
        let primary = Arc::new(AtomicUsize::new(0));
        let fallback = Arc::new(AtomicUsize::new(0));
        let chain = CaptureChain::new(vec![
            Candidate {
                name: "primary",
                method: Box::new(Mock {
                    success: false,
                    calls: primary.clone(),
                }),
            },
            Candidate {
                name: "fallback",
                method: Box::new(Mock {
                    success: true,
                    calls: fallback.clone(),
                }),
            },
        ]);
        chain.capture_frame(&[]).await.unwrap();
        chain.capture_frame(&[]).await.unwrap();
        assert_eq!(primary.load(Ordering::Relaxed), 1);
        assert_eq!(fallback.load(Ordering::Relaxed), 2);
    }

    /// 【捕获测试】【错误诊断】验证所有后端失败时保留每个协议名称与原因
    #[tokio::test]
    async fn failure_preserves_backend_diagnostics() {
        let chain = CaptureChain::new(vec![Candidate {
            name: "native",
            method: Box::new(Mock {
                success: false,
                calls: Arc::new(AtomicUsize::new(0)),
            }),
        }]);
        let error = chain.capture_frame(&[]).await.err().unwrap().to_string();
        assert!(error.contains("native: unavailable"));
    }
}
