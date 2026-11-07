//! 【模型翻译】【响应解析】区分候选、内容块和文本 JSON 的失败原因，保持各服务的原版错误格式

use super::super::{Error, ErrorKind, Provider, Result, response};
use serde_json::Value;
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

/// 【模型翻译】【协议错误】按 detail 返回明确的响应协议错误，不附带完整请求或服务配置
fn protocol(detail: &str) -> Error {
    Error::new(ErrorKind::Protocol, detail)
}

/// 【模型翻译】【业务错误】按 provider 和 error 返回厂商业务错误，空对象沿用原版空字段，任务交付前统一脱敏
fn api_error(provider: Provider, error: &Value) -> Error {
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let detail = if provider == Provider::Anthropic {
        let kind = error
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        format!("anthropic error ({kind}): {message}")
    } else {
        format!("{} error: {message}", provider.id())
    };
    Error::new(ErrorKind::Api, detail)
}

/// 【模型翻译】【首个候选】按 body 提取 Gemini 首候选的首个内容块文本，返回原版分类错误，不拼接其他候选或后续块
fn gemini(body: &Value) -> Result<String> {
    let candidates = body
        .get("candidates")
        .and_then(Value::as_array)
        .filter(|rows| !rows.is_empty())
        .ok_or_else(|| protocol("gemini response missing candidates"))?;
    let parts = candidates[0]
        .pointer("/content/parts")
        .and_then(Value::as_array)
        .filter(|rows| !rows.is_empty())
        .ok_or_else(|| protocol("gemini response candidate missing content parts"))?;
    let text = parts[0]
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if text.trim().is_empty() {
        return Err(protocol("gemini candidate part text is empty"));
    }
    Ok(text.to_owned())
}

/// 【模型翻译】【文本块组合】按 body 拼接 Anthropic 全部 text 块，忽略思考块，返回缺少内容或空正文的明确错误
fn anthropic(body: &Value) -> Result<String> {
    let parts = body
        .get("content")
        .and_then(Value::as_array)
        .filter(|rows| !rows.is_empty())
        .ok_or_else(|| protocol("anthropic response missing content"))?;
    let text: String = parts
        .iter()
        .filter(|part| part.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect();
    if text.trim().is_empty() {
        return Err(protocol("anthropic response text content is empty"));
    }
    Ok(text)
}

/// 【模型翻译】【完整解析】按 provider 和 body 返回身份映射，外层对象、厂商内容和内部翻译数组分别校验
pub(super) fn parse(provider: Provider, body: &Value) -> Result<BTreeMap<i32, String>> {
    if !body.is_object() && matches!(provider, Provider::Gemini | Provider::Anthropic) {
        return Err(protocol(if provider == Provider::Gemini {
            "invalid JSON response from Gemini"
        } else {
            "invalid JSON response from Anthropic"
        }));
    }
    if let Some(error) = body.get("error") {
        return Err(api_error(provider, error));
    }
    let content = match provider {
        Provider::Gemini => gemini(body)?,
        Provider::Anthropic => anthropic(body)?,
        _ => {
            let choices = body
                .get("choices")
                .and_then(Value::as_array)
                .filter(|rows| !rows.is_empty())
                .ok_or_else(|| protocol("llm response missing choices"))?;
            let text = choices[0]
                .pointer("/message/content")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if text.trim().is_empty() {
                return Err(protocol("llm response missing message content"));
            }
            text.to_owned()
        }
    };
    response::content_for(provider, &content)
}
