//! 【模型翻译】【协议差异】共享分段提示，分别构造三类服务的鉴权和响应入口

use super::super::{
    Output, Provider, Result, Segment,
    http::{Request, Transport},
    resolve::Llm,
    response,
};
use serde_json::{Value, json};

/// 【模型翻译】【目标地址】按 config 返回原版版本路径，用户指定的完整 OpenAI 或 Anthropic 路径不重复追加
fn endpoint(config: &Llm) -> String {
    let mut base = config.endpoint.trim_end_matches('/').to_owned();
    match config.provider {
        Provider::OpenAi => {
            if !base.ends_with("/chat/completions") {
                base.push_str("/chat/completions");
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
fn payload(config: &Llm, segments: &[Segment], target: &str) -> Value {
    let prompt = response::prompt(segments, target).to_string();
    match config.provider {
        Provider::OpenAi => {
            let mut body = config.extra_body.clone();
            body.insert("model".into(), json!(config.model));
            body.insert(
                "temperature".into(),
                json!(config.temperature.unwrap_or(0.2)),
            );
            body.insert("messages".into(), json!([
                {"role":"system","content":config.system_prompt}, {"role":"user","content":prompt}]));
            Value::Object(body)
        }
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
    }
}

/// 【模型翻译】【完整执行】按 config、segments、target 和 http 发送一次请求，返回保持输入身份的结果，不修改原文
pub(super) async fn translate(
    config: &Llm,
    segments: &[Segment],
    target: &str,
    http: &dyn Transport,
) -> Result<Output> {
    let headers = match config.provider {
        Provider::OpenAi => vec![("Authorization".into(), format!("Bearer {}", config.key.0))],
        Provider::Gemini => vec![("x-goog-api-key".into(), config.key.0.clone())],
        _ => vec![
            ("x-api-key".into(), config.key.0.clone()),
            ("anthropic-version".into(), "2023-06-01".into()),
        ],
    };
    let body = http
        .send(Request {
            provider: config.provider,
            endpoint: endpoint(config),
            content_type: "application/json",
            headers,
            body: payload(config, segments, target).to_string().into_bytes(),
            timeout: config.timeout,
        })
        .await?;
    Ok(response::mapped(
        config.provider,
        segments,
        &super::model_response::parse(config.provider, &body)?,
    ))
}
