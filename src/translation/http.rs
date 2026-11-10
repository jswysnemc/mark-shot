//! 【翻译服务】【异步传输】限制响应体和重定向，取消任务时直接丢弃网络未来值

use super::{Error, ErrorKind, Provider, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;

const MAX_RESPONSE: usize = 4 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct Request {
    pub provider: Provider,
    pub endpoint: String,
    pub content_type: &'static str,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub timeout: Duration,
}

#[async_trait]
pub(crate) trait Transport: Send + Sync {
    /// 【翻译服务】【请求交付】按 request 发送一次完整 POST，返回有限 JSON 响应或错误，不重试或切换服务
    async fn send(&self, request: Request) -> Result<Value>;
}

pub(crate) struct Http {
    client: reqwest::Client,
}

impl Http {
    /// 【翻译服务】【客户构造】创建按需 HTTP 客户，返回配置错误，无参数；禁止自动重定向和空闲连接常驻
    pub fn new() -> Result<Self> {
        reqwest::Client::builder()
            .hickory_dns(true)
            .redirect(reqwest::redirect::Policy::none())
            .pool_max_idle_per_host(0)
            .http1_only()
            .user_agent(concat!("Mark-Shot/", env!("CARGO_PKG_VERSION")))
            .build()
            .map(|client| Self { client })
            .map_err(|_| {
                Error::new(
                    ErrorKind::Network,
                    "failed to initialize translation HTTP client",
                )
            })
    }
    /// 【翻译服务】【模型查询】用 request 发起有界 GET，返回 JSON，复用安全验证和响应限制
    pub(crate) async fn get(&self, request: Request) -> Result<Value> {
        self.execute(request, reqwest::Method::GET).await
    }

    /// 【翻译服务】【网络响应】发送 request 并逐块读取，返回 JSON；限制响应内存，不在错误中包含带凭据的 URL
    async fn execute(&self, request: Request, method: reqwest::Method) -> Result<Value> {
        let endpoint = url::Url::parse(&request.endpoint)
            .map_err(|_| Error::new(ErrorKind::Configuration, "invalid translation endpoint"))?;
        if !matches!(endpoint.scheme(), "http" | "https")
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
        {
            return Err(Error::new(
                ErrorKind::Configuration,
                "translation endpoint must use HTTP(S) without URL credentials",
            ));
        }
        // 1. 【翻译服务】【请求组装】请求头不参与调试格式，服务签名覆盖的正文保持原始字节
        let mut builder = self
            .client
            .request(method, endpoint)
            .timeout(request.timeout)
            .header(reqwest::header::CONTENT_TYPE, request.content_type)
            .body(request.body);
        for (name, value) in request.headers {
            builder = builder.header(name, value);
        }
        let mut response = builder.send().await.map_err(network_error)?;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE as u64)
        {
            return Err(Error::new(
                ErrorKind::Protocol,
                "translation response exceeds four MiB",
            ));
        }
        let status = response.status();
        let mut bytes = Vec::new();
        // 2. 【翻译服务】【有限读取】即使缺少长度声明，正文也不能超过上限；未来值销毁后不继续读取
        while let Some(chunk) = response.chunk().await.map_err(network_error)? {
            if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE {
                return Err(Error::new(
                    ErrorKind::Protocol,
                    "translation response exceeds four MiB",
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        if !status.is_success()
            && (!matches!(
                request.provider,
                Provider::Tencent | Provider::Baidu | Provider::Youdao
            ) || status.is_redirection())
        {
            let detail = if matches!(request.provider, Provider::Gemini | Provider::Anthropic) {
                serde_json::from_slice::<Value>(&bytes)
                    .ok()
                    .filter(|root| root.get("error").is_some())
                    .map(|root| {
                        root.pointer("/error/message")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned()
                    })
            } else {
                None
            }
            .unwrap_or_else(|| {
                String::from_utf8_lossy(&bytes[..bytes.len().min(500)]).into_owned()
            });
            return Err(Error::new(
                ErrorKind::Http,
                format!(
                    "{} http {}: {}",
                    if request.provider == Provider::OpenAi {
                        "llm"
                    } else {
                        request.provider.id()
                    },
                    status.as_u16(),
                    detail
                ),
            ));
        }
        serde_json::from_slice(&bytes).map_err(|_| {
            Error::new(
                ErrorKind::Protocol,
                match request.provider {
                    Provider::Gemini => "invalid JSON response from Gemini",
                    Provider::Anthropic => "invalid JSON response from Anthropic",
                    _ => "translation response is not valid JSON",
                },
            )
        })
    }
}

#[async_trait]
impl Transport for Http {
    /// 【翻译服务】【翻译请求】按 request 发起 POST，返回经过限额验证的 JSON 响应
    async fn send(&self, request: Request) -> Result<Value> {
        self.execute(request, reqwest::Method::POST).await
    }
}

/// 【翻译服务】【传输错误】按 error 返回超时或网络类别，不输出完整请求、URL、请求头或内部凭据
fn network_error(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::new(ErrorKind::Timeout, "timed out")
    } else {
        let detail = if error.is_connect() {
            "无法连接翻译服务，请检查网络或代理 / Cannot connect to translation service"
        } else if error.is_body() || error.is_decode() {
            "翻译响应读取中断 / Translation response was interrupted"
        } else {
            "翻译请求传输失败 / Translation request transport failed"
        };
        Error::new(ErrorKind::Network, detail)
    }
}
