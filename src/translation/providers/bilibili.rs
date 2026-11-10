//! 【哔哩哔哩翻译】【临时故障】在同一服务及总超时内重试连接失败，不切换模型或泄露请求

use crate::translation::{
    Error, ErrorKind, Result,
    http::{Request, Transport},
};
use serde_json::Value;
use std::time::{Duration, Instant};

/// 【哔哩哔哩翻译】【重试判断】按 error 返回是否属于临时连接或网关故障，认证、模型参数和内容错误不重试
fn transient(error: &Error) -> bool {
    error.kind == ErrorKind::Network
        || error.kind == ErrorKind::Http
            && [502, 503, 504].into_iter().any(|status| {
                error
                    .detail
                    .starts_with(&format!("bilibili http {status}:"))
            })
}

/// 【哔哩哔哩翻译】【有界重试】通过 http 发送 request，最多三次且共用总超时，返回首次成功或最终错误；取消时立即释放等待
pub(super) async fn send(http: &dyn Transport, mut request: Request) -> Result<Value> {
    let deadline = Instant::now() + request.timeout;
    for attempt in 0..3 {
        request.timeout = deadline.saturating_duration_since(Instant::now());
        if request.timeout.is_zero() {
            return Err(Error::new(ErrorKind::Timeout, "timed out"));
        }
        let error = match http.send(request.clone()).await {
            Ok(value) => return Ok(value),
            Err(error) => error,
        };
        let delay = Duration::from_millis(250 * (attempt + 1));
        if attempt == 2 || !transient(&error) || Instant::now() + delay >= deadline {
            return Err(error);
        }
        log::info!(
            "【哔哩哔哩翻译】【临时故障】第 {} 次请求失败，准备重试",
            attempt + 1
        );
        tokio::time::sleep(delay).await;
    }
    unreachable!()
}
