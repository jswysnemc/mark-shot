//! 【翻译服务】【服务组合】选择已解析的原生实现，不在调用失败后自动向其他服务发送原文

mod baidu;
mod bilibili;
pub(crate) mod llm;
mod model_response;
mod tc3;
mod tencent;
mod youdao;

use super::{Output, Result, Segment, http::Transport, resolve::Service};

/// 【翻译服务】【实际执行】按 service、segments、target 和 http 请求已选服务，返回完整输出；错误统一脱敏
pub(crate) async fn translate(
    service: &Service,
    segments: &[Segment],
    target: &str,
    http: &dyn Transport,
) -> Result<Output> {
    let result = match service {
        Service::Llm(config) => llm::translate(config, segments, target, http).await,
        Service::Tencent(config) => tencent::translate(config, segments, target, http).await,
        Service::Form(config) if config.provider == super::Provider::Baidu => {
            baidu::translate(config, segments, target, http).await
        }
        Service::Form(config) => youdao::translate(config, segments, target, http).await,
    };
    result.map_err(|mut error| {
        service.redact(&mut error);
        error
    })
}

/// 【翻译服务】【批次压平】按 segments 返回替换段内制表符和换行的副本，不改变输入原文或身份
fn flattened(segments: &[Segment]) -> Vec<Segment> {
    segments
        .iter()
        .map(|segment| Segment {
            id: segment.id,
            text: segment
                .text
                .replace(['\r', '\n', '\t'], " ")
                .trim()
                .to_owned(),
        })
        .collect()
}

/// 【翻译服务】【字符预算】按 segments 和 limit 返回有序切片批次，字符数沿用原版 UTF-16 计量，超长单段独立成批
fn batches(segments: &[Segment], limit: usize) -> Vec<&[Segment]> {
    let mut result = Vec::new();
    let (mut start, mut count) = (0, 0usize);
    for (index, segment) in segments.iter().enumerate() {
        let length = segment.text.encode_utf16().count();
        if index > start && count.saturating_add(length) > limit {
            result.push(&segments[start..index]);
            start = index;
            count = 0;
        }
        count = count.saturating_add(length);
    }
    if start < segments.len() {
        result.push(&segments[start..]);
    }
    result
}

/// 【翻译服务】【请求随机数】返回十六个系统随机字节，无参数；失败不以时间或进程编号代替随机数
fn random() -> Result<[u8; 16]> {
    let mut value = [0; 16];
    getrandom::fill(&mut value).map_err(|_| {
        super::Error::new(
            super::ErrorKind::StartFailed,
            "cannot generate translation request salt",
        )
    })?;
    Ok(value)
}

/// 【翻译服务】【十六进制】按 bytes 返回小写十六进制字符串，保持二进制签名中间量与最终文本分离
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests;

/// 【翻译服务】【原版编码】按 value 与 plus_spaces 返回 Qt 百分号编码，修正 HTML 表单对星号和波浪线的不同规则
fn encode(value: &str, plus_spaces: bool) -> String {
    let encoded: String = url::form_urlencoded::byte_serialize(value.as_bytes()).collect();
    let encoded = encoded.replace('*', "%2A").replace("%7E", "~");
    if plus_spaces {
        encoded
    } else {
        encoded.replace('+', "%20")
    }
}
