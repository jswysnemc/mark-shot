//! 【图片上传】【内置传输】兼容原版 multipart 环境字段与常用图床响应，限制响应体，不打印服务凭据

use super::{Error, ErrorKind, MAX_IMAGE, MAX_RESPONSE, Output, Result};
use crate::config::upload::Upload;
use serde_json::Value;
use std::collections::BTreeMap;

pub const BOUNDARY: &str = "----mark-shot-upload-boundary";

/// 【图片上传】【环境默认】按 env、name 和 default 返回去除首尾空白的非空环境值，未配置时使用原版默认
fn variable<'a>(env: &'a BTreeMap<String, String>, name: &str, default: &'a str) -> &'a str {
    env.get(name)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(default)
}

/// 【图片上传】【字段引用】按 value 返回 multipart 引用值，拒绝换行、控制字符和过大名称，返回字段或配置错误
fn field_name(value: &str) -> Result<String> {
    if value.is_empty() || value.len() > 1024 || value.chars().any(char::is_control) {
        return Err(Error::new(ErrorKind::Configuration, "Invalid upload field"));
    }
    Ok(value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// 【图片上传】【正文组装】按 config、png 和 filename 返回完整 multipart，字段顺序稳定，不改变 PNG 字节
pub fn multipart(config: &Upload, png: &[u8], filename: &str) -> Result<Vec<u8>> {
    if png.is_empty() || png.len() > MAX_IMAGE {
        return Err(Error::new(
            ErrorKind::Configuration,
            "Upload image exceeds limit or is empty",
        ));
    }
    let name = field_name(variable(&config.env, "MARK_SHOT_UPLOAD_FIELD", "image"))?;
    let filename = field_name(filename)?;
    let mut body = Vec::new();
    for (key, value) in &config.env {
        if let Some(key) = key.strip_prefix("MARK_SHOT_UPLOAD_FIELD_") {
            let key = field_name(key)?;
            body.extend_from_slice(format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{key}\"\r\n\r\n{value}\r\n").as_bytes());
        }
    }
    body.extend_from_slice(format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: image/png\r\n\r\n").as_bytes());
    body.extend_from_slice(png);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    Ok(body)
}

/// 【图片上传】【嵌套字段】按 root 和 path 提取点分隔对象或数组字段，返回字符串；越界、负数和错误类型返回空值
pub fn extract(root: &Value, path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    let mut current = root;
    for part in path.split('.') {
        current = if let Some(array) = current.as_array() {
            let Ok(index) = part.parse::<usize>() else {
                return String::new();
            };
            let Some(value) = array.get(index) else {
                return String::new();
            };
            value
        } else {
            let Some(value) = current.as_object().and_then(|object| object.get(part)) else {
                return String::new();
            };
            value
        };
    }
    current.as_str().unwrap_or_default().to_owned()
}

/// 【图片上传】【结果解析】按 bytes、url_path、delete_path 和 command 返回原版 URL 结果；命令 JSON 只读取顶层 url
pub fn output(bytes: &[u8], url_path: &str, delete_path: &str, command: bool) -> Result<Output> {
    if bytes.len() > MAX_RESPONSE {
        return Err(Error::new(
            ErrorKind::Protocol,
            "Upload response exceeds limit",
        ));
    }
    let text = String::from_utf8_lossy(bytes);
    let root = serde_json::from_slice::<Value>(bytes).ok();
    let (url, delete_url) = if let Some(root) = root.filter(Value::is_object) {
        if command {
            (extract(&root, "url"), String::new())
        } else {
            let url = std::iter::once(url_path)
                .chain(["url", "data.url", "data.link", "link", "image.url"])
                .map(|path| extract(&root, path))
                .find(|value| !value.is_empty())
                .unwrap_or_default();
            let delete = std::iter::once(delete_path)
                .chain(["delete", "data.delete_url", "data.delete"])
                .map(|path| extract(&root, path))
                .find(|value| !value.is_empty())
                .unwrap_or_default();
            (url, delete)
        }
    } else {
        let first = text.trim().lines().next().unwrap_or_default().trim();
        let prefix = if command {
            first.to_ascii_lowercase()
        } else {
            first.to_owned()
        };
        if !prefix.starts_with("http://") && !prefix.starts_with("https://") {
            return Err(Error::new(ErrorKind::Protocol, "Upload failed"));
        }
        (first.to_owned(), String::new())
    };
    // 1. 【图片上传】【原版返回值】JSON 服务只要求 url 字符串非空，不额外限制地址协议或改写用户返回内容
    let url = url.trim().to_owned();
    if url.is_empty() {
        return Err(Error::new(ErrorKind::Protocol, "Upload failed"));
    }
    Ok(Output { url, delete_url })
}

/// 【图片上传】【网络错误】按 error 返回超时或网络错误，屏蔽完整 URL、响应正文和请求凭据
fn network_error(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::new(ErrorKind::Timeout, "Upload timed out")
    } else {
        Error::new(ErrorKind::Network, "Upload failed")
    }
}

/// 【图片上传】【HTTP执行】按 config、png 发送单次 POST，返回有限 URL 响应；调用方控制任务取消与总超时
pub async fn send(config: &Upload, png: &[u8]) -> Result<Output> {
    let endpoint = variable(&config.env, "MARK_SHOT_UPLOAD_URL", "");
    let endpoint = url::Url::parse(endpoint).map_err(|_| {
        Error::new(
            ErrorKind::Configuration,
            "Upload endpoint is not configured",
        )
    })?;
    if !matches!(endpoint.scheme(), "http" | "https")
        || endpoint.host_str().is_none()
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
    {
        return Err(Error::new(
            ErrorKind::Configuration,
            "Invalid upload endpoint",
        ));
    }
    let client = reqwest::Client::builder()
        .hickory_dns(true)
        .pool_max_idle_per_host(0)
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(network_error)?;
    let mut headers = reqwest::header::HeaderMap::new();
    let key = variable(&config.env, "MARK_SHOT_UPLOAD_API_KEY", "");
    if !key.is_empty() {
        let name = variable(&config.env, "MARK_SHOT_UPLOAD_AUTH_HEADER", "Authorization");
        let scheme = variable(&config.env, "MARK_SHOT_UPLOAD_AUTH_SCHEME", "Bearer");
        insert_header(&mut headers, name, &format!("{scheme} {key}"))?;
    }
    for (name, value) in &config.env {
        if let Some(name) = name.strip_prefix("MARK_SHOT_UPLOAD_HEADER_") {
            insert_header(&mut headers, name, value)?;
        }
    }
    headers.entry(reqwest::header::CONTENT_TYPE).or_insert(
        reqwest::header::HeaderValue::from_static(
            "multipart/form-data; boundary=----mark-shot-upload-boundary",
        ),
    );
    // 1. 【图片上传】【私有正文】HTTP 模式直接交付最终 PNG，不在临时目录写入用户图片
    let mut response = client
        .post(endpoint)
        .headers(headers)
        .body(multipart(config, png, "screenshot.png")?)
        .timeout(std::time::Duration::from_millis(config.timeout_ms))
        .send()
        .await
        .map_err(network_error)?;
    if !response.status().is_success() {
        return Err(Error::new(
            ErrorKind::Http,
            format!("Upload failed (HTTP {})", response.status().as_u16()),
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE as u64)
    {
        return Err(Error::new(
            ErrorKind::Protocol,
            "Upload response exceeds limit",
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(network_error)? {
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE {
            return Err(Error::new(
                ErrorKind::Protocol,
                "Upload response exceeds limit",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    output(
        &bytes,
        variable(&config.env, "MARK_SHOT_UPLOAD_URL_PATH", ""),
        variable(&config.env, "MARK_SHOT_UPLOAD_DELETE_URL_PATH", ""),
        false,
    )
}

/// 【图片上传】【请求头校验】把 name、value 加入 headers，返回校验结果；配置错误不回显凭据值
fn insert_header(headers: &mut reqwest::header::HeaderMap, name: &str, value: &str) -> Result<()> {
    let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
        .map_err(|_| Error::new(ErrorKind::Configuration, "Invalid upload header"))?;
    let value = reqwest::header::HeaderValue::from_str(value)
        .map_err(|_| Error::new(ErrorKind::Configuration, "Invalid upload header"))?;
    headers.insert(name, value);
    Ok(())
}
