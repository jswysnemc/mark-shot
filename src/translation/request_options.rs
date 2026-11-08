//! 【翻译服务】【请求选项】统一自定义正文、思考开关和正文过滤

use super::{Error, ErrorKind, Provider, Result, resolve::Llm};
use serde_json::{Map, Value, json};

/// 【翻译服务】【扩展校验】按 value 解析对象或 JSON 字符串，返回字段映射；错误不回显原始内容
pub(crate) fn parse_extra(value: Option<&Value>) -> Result<Map<String, Value>> {
    let parsed = match value {
        None | Some(Value::Null) => return Ok(Map::new()),
        Some(Value::String(text)) if text.trim().is_empty() => return Ok(Map::new()),
        Some(Value::String(text)) if text.len() <= 65536 => serde_json::from_str(text).ok(),
        Some(Value::Object(object)) => return Ok(object.clone()),
        _ => None,
    };
    parsed
        .and_then(|value: Value| value.as_object().cloned())
        .ok_or_else(|| {
            Error::new(
                ErrorKind::Configuration,
                "自定义请求体必须是 JSON 对象 / Custom body must be a JSON object",
            )
        })
}

/// 【翻译服务】【递归覆盖】将 extra 合并到 body，返回空值；对象递归覆盖，数组和空值整体替换
pub(crate) fn merge(body: &mut Value, extra: &Value) {
    if let (Some(target), Some(source)) = (body.as_object_mut(), extra.as_object()) {
        for (key, value) in source {
            merge(target.entry(key.clone()).or_insert(Value::Null), value);
        }
    } else {
        *body = extra.clone();
    }
}

/// 【翻译服务】【思考设置】按 config 为 body 增加协议选项，无返回值；模型不支持时由服务明确报错
pub(crate) fn thinking(config: &Llm, body: &mut Value) {
    if !config.disable_thinking {
        return;
    }
    let options = match config.provider {
        Provider::Bilibili => json!({"chat_template_kwargs":{"enable_thinking":false}}),
        Provider::OpenAi => json!({"reasoning_effort":"none"}),
        Provider::Responses => json!({"reasoning":{"effort":"none"}}),
        Provider::Anthropic => json!({"thinking":{"type":"disabled"}}),
        Provider::Gemini => json!({"generationConfig":{"thinkingConfig":{"thinkingBudget":0}}}),
        _ => return,
    };
    if config.provider == Provider::Gemini {
        body["generationConfig"]
            .as_object_mut()
            .unwrap()
            .remove("thinkingConfig");
    }
    merge(body, &options);
}

/// 【翻译服务】【思考过滤】从 text 去除成对或未闭合的 think、thinking 标签，返回剩余正文，保留标签外原文
pub(crate) fn filter(text: &str) -> String {
    let mut result = text.to_owned();
    for tag in ["think", "thinking"] {
        let opening = format!("<{tag}>");
        let closing = format!("</{tag}>");
        loop {
            let lower = result.to_ascii_lowercase();
            let Some(start) = lower.find(&opening) else {
                break;
            };
            let end = lower[start + opening.len()..]
                .find(&closing)
                .map_or(result.len(), |offset| {
                    start + opening.len() + offset + closing.len()
                });
            result.replace_range(start..end, "");
        }
    }
    result.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【翻译选项测试】【协议请求】无参数，验证默认服务、必填字段保护及不同思考参数，返回空值
    #[test]
    fn default_and_protocol_options_preserve_translation_input() {
        let automatic = super::super::resolve::service(&json!({}), "auto", &|_| None).unwrap();
        assert_eq!(automatic.provider(), Provider::Bilibili);
        for (id, vendor, pointer, expected) in [
            (
                "bilibili",
                "bilibili",
                "/chat_template_kwargs/enable_thinking",
                json!(false),
            ),
            (
                "openai-compatible",
                "openai",
                "/reasoning_effort",
                json!("none"),
            ),
            (
                "openai-responses",
                "responses",
                "/reasoning/effort",
                json!("none"),
            ),
            (
                "anthropic",
                "anthropic",
                "/thinking/type",
                json!("disabled"),
            ),
            (
                "gemini",
                "gemini",
                "/generationConfig/thinkingConfig/thinkingBudget",
                json!(0),
            ),
        ] {
            let root = json!({vendor:{"apiKey":"test","disableThinking":true,"extraBody":{
                "model":"wrong","messages":[],"input":[],"contents":[],"stream":true,"custom":null}}});
            let super::super::resolve::Service::Llm(config) =
                super::super::resolve::service(&root, &format!("plugin:{id}"), &|_| None).unwrap()
            else {
                panic!()
            };
            let body = super::super::providers::llm::payload(
                &config,
                &[super::super::Segment {
                    id: 3,
                    text: "source".into(),
                }],
                "Japanese",
            );
            assert_eq!(body.pointer(pointer), Some(&expected));
            assert_ne!(body["model"], "wrong");
            if id != "gemini" {
                assert_eq!(body["stream"], false);
            }
            assert!(body.to_string().contains("source"));
        }
    }

    /// 【翻译选项测试】【正文过滤】无参数，验证思考标签、UTF-8 和扩展 JSON 对象，返回空值
    #[test]
    fn reasoning_filter_preserves_unicode_and_json() {
        assert_eq!(
            filter("<THINK>{\"wrong\":1}</THINK>\n{\"translations\":[]}"),
            "{\"translations\":[]}"
        );
        assert_eq!(filter("译文<thinking>private"), "译文");
        assert_eq!(filter("<think>one</think><think>two</think>译文"), "译文");
        assert!(parse_extra(Some(&json!("[]"))).is_err());
        assert!(
            parse_extra(Some(&json!("{\"temperature\":null}"))).unwrap()["temperature"].is_null()
        );
    }
}
