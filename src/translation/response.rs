//! 【翻译服务】【结果映射】解析模型 JSON，按原输入身份回填，缺失译文沿用原版原文回退

use super::{Error, ErrorKind, Output, Provider, Result, Segment};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// 【翻译服务】【用户请求】按 segments 和 target 返回原版模型用户消息，不改变分段数和身份
pub(crate) fn prompt(segments: &[Segment], target: &str) -> Value {
    json!({"target_language": if target.trim().is_empty() { "Simplified Chinese" } else { target.trim() },
        "instructions": ["Translate each segment into target_language.",
            "Return JSON exactly as {\"translations\":[{\"id\":0,\"text\":\"...\"}]} with no markdown.", "Do not add explanations."],
        "segments": segments})
}

/// 【翻译服务】【模型内容】按 content 读取 translations 数组，返回身份映射，兼容完整 JSON 外的 Markdown 包裹
#[cfg(test)]
pub(crate) fn content(content: &str) -> Result<BTreeMap<i32, String>> {
    content_for(Provider::OpenAi, content)
}

/// 【翻译服务】【模型错误格式】按 provider 和 content 返回身份映射，解析规则共用，失败文案沿用对应原版服务
pub(crate) fn content_for(provider: Provider, content: &str) -> Result<BTreeMap<i32, String>> {
    let parsed = serde_json::from_str::<Value>(content.trim())
        .ok()
        .filter(Value::is_object)
        .or_else(|| {
            let start = content.find('{')?;
            let end = content.rfind('}')?;
            (end >= start)
                .then(|| serde_json::from_str::<Value>(&content[start..=end]).ok())
                .flatten()
                .filter(Value::is_object)
        })
        .ok_or_else(|| {
            let detail = if matches!(provider, Provider::Gemini | Provider::Anthropic) {
                let units: Vec<_> = content.trim().encode_utf16().take(200).collect();
                let prefix: String = char::decode_utf16(units)
                    .filter_map(std::result::Result::ok)
                    .collect();
                format!(
                    "{} text response is not valid JSON: {prefix}",
                    provider.id()
                )
            } else {
                "llm response is not valid JSON".into()
            };
            Error::new(ErrorKind::Protocol, detail)
        })?;
    let translations = parsed
        .get("translations")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            Error::new(
                ErrorKind::Protocol,
                format!(
                    "{} response missing translations array",
                    if provider == Provider::OpenAi {
                        "llm"
                    } else {
                        provider.id()
                    }
                ),
            )
        })?;
    let mut result = BTreeMap::new();
    for entry in translations {
        if let Some(value) = entry.get("id") {
            // 1. 【翻译服务】【原版整数】整数浮点数可以转换，其他类型、非整数和越界值沿用 Qt 默认零
            let id = value
                .as_i64()
                .and_then(|id| i32::try_from(id).ok())
                .or_else(|| {
                    value
                        .as_f64()
                        .filter(|id| {
                            id.fract() == 0.0 && *id >= i32::MIN as f64 && *id <= i32::MAX as f64
                        })
                        .map(|id| id as i32)
                })
                .unwrap_or(0);
            result.insert(
                id,
                entry
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_owned(),
            );
        }
    }
    Ok(result)
}

/// 【翻译服务】【有序回填】按 provider、segments 和 translations 返回完整输出，重复身份按最后结果处理，空译文保留原文
pub(crate) fn mapped(
    provider: Provider,
    segments: &[Segment],
    translations: &BTreeMap<i32, String>,
) -> Output {
    Output {
        provider,
        segments: segments
            .iter()
            .map(|segment| Segment {
                id: segment.id,
                text: translations
                    .get(&segment.id)
                    .map(String::as_str)
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                    .unwrap_or(&segment.text)
                    .to_owned(),
            })
            .collect(),
    }
}

/// 【翻译服务】【批次数量】按 provider、segments 和 texts 校验条数并返回未回填译文，避免压平请求文本污染原文回退
pub(crate) fn checked(
    provider: Provider,
    segments: &[Segment],
    texts: Vec<String>,
) -> Result<Vec<String>> {
    if segments.len() != texts.len() {
        return Err(Error::new(
            ErrorKind::Protocol,
            format!(
                "{} returned {} translations for {} source texts",
                provider.id(),
                texts.len(),
                segments.len()
            ),
        ));
    }
    Ok(texts)
}

/// 【翻译服务】【批次对应】按 provider、原始 segments 和 texts 返回完整输出，条数不符立即失败，空译文只在最终结果回退
pub(crate) fn ordered(
    provider: Provider,
    segments: &[Segment],
    texts: Vec<String>,
) -> Result<Output> {
    let texts = checked(provider, segments, texts)?;
    Ok(Output {
        provider,
        segments: segments
            .iter()
            .zip(texts)
            .map(|(source, text)| Segment {
                id: source.id,
                text: if text.trim().is_empty() {
                    source.text.clone()
                } else {
                    text.trim().to_owned()
                },
            })
            .collect(),
    })
}

/// 【翻译服务】【错误字段】按 value 返回字符串或整数错误码，其他类型返回空字符串，不序列化整个错误对象
pub(crate) fn code(value: Option<&Value>) -> String {
    value
        .and_then(|value| match value {
            Value::String(text) => Some(text.trim().to_owned()),
            Value::Number(number) => Some(number.to_string()),
            _ => None,
        })
        .unwrap_or_default()
}
