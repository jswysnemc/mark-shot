//! 【模型翻译】【协议差异】共享分段提示，分别构造三类服务的鉴权和响应入口

use super::super::{
    Output, Provider, Result, Segment,
    http::{Request, Transport},
    request_options,
    resolve::Llm,
    response,
};
use serde_json::{Value, json};

/// 【模型翻译】【目标地址】按 config 返回原版版本路径，用户指定的完整 OpenAI 或 Anthropic 路径不重复追加
pub(crate) fn endpoint(config: &Llm) -> String {
    let mut base = config.endpoint.trim_end_matches('/').to_owned();
    match config.provider {
        Provider::OpenAi | Provider::Bilibili => {
            if !base.ends_with("/chat/completions") {
                base.push_str("/chat/completions");
            }
        }
        Provider::Responses => {
            if !base.ends_with("/responses") {
                base.push_str("/responses");
            }
        }
        Provider::Gemini => {
            if !base.contains("/v1") {
                base.push_str("/v1beta");
            }
            base.push_str(&format!(
                "/models/{}:generateContent",
                config.model.trim().trim_start_matches("models/")
            ));
        }
        _ => {
            if !base.ends_with("/messages") {
                if !base.ends_with("/v1") && !base.contains("/v1/") {
                    base.push_str("/v1");
                }
                base.push_str("/messages");
            }
        }
    }
    base
}

/// 【模型翻译】【服务正文】按 config、segments 和 target 构造 JSON，必需字段覆盖 OpenAI extraBody，不给其他服务添加默认温度
pub(crate) fn payload(config: &Llm, segments: &[Segment], target: &str) -> Value {
    let prompt = response::prompt(segments, target).to_string();
    let mut body = match config.provider {
        Provider::OpenAi | Provider::Bilibili => {
            let mut body = serde_json::Map::new();
            body.insert("model".into(), json!(config.model));
            body.insert(
                "temperature".into(),
                json!(config.temperature.unwrap_or(0.2)),
            );
            body.insert("messages".into(), json!([
                {"role":"system","content":config.system_prompt}, {"role":"user","content":prompt}]));
            Value::Object(body)
        }
        Provider::Responses => json!({"model":config.model,"instructions":config.system_prompt,
            "input":[{"role":"user","content":prompt}], "store":false}),
        Provider::Gemini => {
            let mut body = json!({"contents":[{"role":"user","parts":[{"text":prompt}]}],
                "generationConfig":{"responseMimeType":"application/json"}});
            if !config.system_prompt.is_empty() {
                body["systemInstruction"] = json!({"parts":[{"text":config.system_prompt}]});
            }
            if let Some(temperature) = config.temperature {
                body["generationConfig"]["temperature"] = json!(temperature);
            }
            if !config.thinking_level.is_empty() {
                body["generationConfig"]["thinkingConfig"] =
                    json!({"thinkingLevel":config.thinking_level.trim().to_uppercase()});
            }
            body
        }
        _ => {
            let mut body = json!({"model":config.model,"max_tokens":config.max_tokens,"messages":[{"role":"user","content":prompt}]});
            if !config.system_prompt.is_empty() {
                body["system"] = json!(config.system_prompt);
            }
            if let Some(temperature) = config.temperature {
                body["temperature"] = json!(temperature);
            }
            body
        }
    };
    request_options::thinking(config, &mut body);
    let required = body.clone();
    request_options::merge(&mut body, &Value::Object(config.extra_body.clone()));
    // 1. 【模型翻译】【身份保护】扩展可以覆盖生成选项，不能替换待翻译文本、模型和同步响应模式
    for field in ["model", "messages", "contents", "input"] {
        if let Some(value) = required.get(field) {
            body[field] = value.clone();
        } else {
            body.as_object_mut().unwrap().remove(field);
        }
    }
    if config.provider != Provider::Gemini {
        body["stream"] = json!(false);
    } else {
        body.as_object_mut().unwrap().remove("stream");
    }
    body
}

/// 【模型翻译】【请求鉴权】按 config 返回对应协议的请求头，不附加浏览器来源信息
pub(crate) fn headers(config: &Llm) -> Vec<(String, String)> {
    match config.provider {
        Provider::OpenAi | Provider::Bilibili | Provider::Responses => {
            vec![("Authorization".into(), format!("Bearer {}", config.key.0))]
        }
        Provider::Gemini => vec![("x-goog-api-key".into(), config.key.0.clone())],
        _ => vec![
            ("x-api-key".into(), config.key.0.clone()),
            ("anthropic-version".into(), "2023-06-01".into()),
        ],
    }
}

/// 【模型翻译】【完整执行】按 config、segments、target 和 http 发送一次请求，返回保持输入身份的结果，不修改原文
pub(super) async fn translate(
    config: &Llm,
    segments: &[Segment],
    target: &str,
    http: &dyn Transport,
) -> Result<Output> {
    let headers = headers(config);
    let request = Request {
        provider: config.provider,
        endpoint: endpoint(config),
        content_type: "application/json",
        headers,
        body: payload(config, segments, target).to_string().into_bytes(),
        timeout: config.timeout,
    };
    let body = if config.provider == Provider::Bilibili {
        super::bilibili::send(http, request).await?
    } else {
        http.send(request).await?
    };
    response::mapped(
        config.provider,
        segments,
        &super::model_response::parse_options(config.provider, &body, config.filter_thinking)?,
    )
}
