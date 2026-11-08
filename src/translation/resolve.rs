//! 【翻译服务】【配置解析】厂商凭据独立解析，保持原版优先级，敏感值不进入调试输出

use super::{Error, ErrorKind, Provider, Result};
use serde_json::{Map, Value};
use std::{fmt, time::Duration};

pub(crate) const PROMPT: &str = "You are a professional native translator. Fluently translate OCR text segments into the target language. Preserve original meaning, keep segment count and ids unchanged. Keep proper nouns, brand names, code snippets, formulas, and numbers intact. Return only valid JSON without extra text or explanations.";

#[derive(Clone)]
pub(crate) struct Secret(pub String);

impl fmt::Debug for Secret {
    /// 【翻译服务】【凭据隐藏】写入固定脱敏标记到 formatter，返回格式结果，不输出密钥长度或内容
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[redacted]")
    }
}

#[derive(Clone)]
pub(crate) struct Llm {
    pub provider: Provider,
    pub endpoint: String,
    pub key: Secret,
    pub model: String,
    pub system_prompt: String,
    pub temperature: Option<f64>,
    pub thinking_level: String,
    pub max_tokens: u64,
    pub extra_body: Map<String, Value>,
    pub filter_thinking: bool,
    pub disable_thinking: bool,
    pub timeout: Duration,
}

#[derive(Clone)]
pub(crate) struct Tencent {
    pub endpoint: String,
    pub secret_id: Secret,
    pub secret_key: Secret,
    pub region: String,
    pub project_id: u64,
    pub timeout: Duration,
}

#[derive(Clone)]
pub(crate) struct FormService {
    pub provider: Provider,
    pub endpoint: String,
    pub app_id: Secret,
    pub key: Secret,
    pub timeout: Duration,
}

#[derive(Clone)]
pub(crate) enum Service {
    Llm(Llm),
    Tencent(Tencent),
    Form(FormService),
}

impl fmt::Debug for Service {
    /// 【翻译服务】【服务脱敏】向 formatter 只写入服务标识，返回格式结果，不输出接入点、额外正文或凭据
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Service")
            .field("provider", &self.provider())
            .finish()
    }
}

impl Service {
    /// 【翻译服务】【当前服务】返回已经解析的稳定服务标识，无参数，不再次读取配置或环境
    pub fn provider(&self) -> Provider {
        match self {
            Self::Llm(config) => config.provider,
            Self::Tencent(_) => Provider::Tencent,
            Self::Form(config) => config.provider,
        }
    }

    /// 【翻译服务】【错误脱敏】删除 error 中可能由上游回显的所有当前凭据，无返回值，不改变业务错误类别
    pub fn redact(&self, error: &mut Error) {
        error.provider = Some(self.provider());
        let secrets: Vec<&str> = match self {
            Self::Llm(config) => vec![&config.key.0],
            Self::Tencent(config) => vec![&config.secret_id.0, &config.secret_key.0],
            Self::Form(config) => vec![&config.app_id.0, &config.key.0],
        };
        for secret in secrets.into_iter().filter(|secret| !secret.is_empty()) {
            error.detail = error.detail.replace(secret, "[redacted]");
        }
    }
}

/// 【翻译服务】【字段兼容】按 object 和 camel key 返回原版驼峰或原生下划线字段，未配置返回空值
fn field<'a>(object: &'a Value, key: &str) -> Option<&'a Value> {
    let mut snake = String::new();
    for ch in key.chars() {
        if ch.is_ascii_uppercase() {
            snake.push('_');
        }
        snake.push(ch.to_ascii_lowercase());
    }
    object.get(key).or_else(|| object.get(&snake))
}

/// 【翻译服务】【字符串优先级】按 vendor、shared、key、环境候选和 fallback 返回首个非空值，env 不改变进程环境
fn string(
    vendor: &Value,
    shared: &Value,
    key: &str,
    env: &dyn Fn(&str) -> Option<String>,
    names: &[&str],
    fallback: &str,
) -> String {
    [field(vendor, key), field(shared, key)]
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            names
                .iter()
                .filter_map(|name| env(name))
                .map(|value| value.trim().to_owned())
                .find(|value| !value.is_empty())
        })
        .unwrap_or_else(|| fallback.trim().to_owned())
}

/// 【翻译服务】【整数范围】按 vendor、shared 和 key 返回有界整数，缺失或类型不符使用 fallback
fn integer(vendor: &Value, shared: &Value, key: &str, fallback: u64, min: u64, max: u64) -> u64 {
    let number = |object: &Value| field(object, key).and_then(Value::as_i64);
    number(vendor)
        .or_else(|| number(shared))
        .map_or(fallback, |value| value.max(0) as u64)
        .clamp(min, max)
}

/// 【翻译服务】【必填配置】按 value 与 name 校验非空字段，返回配置错误，不把字段内容写入错误
fn required(value: &str, name: &str) -> Result<()> {
    if value.is_empty() {
        Err(Error::new(
            ErrorKind::Configuration,
            format!("missing {name}"),
        ))
    } else {
        Ok(())
    }
}

/// 【翻译服务】【单一配置】按 root、provider 和 env 解析服务，返回完整配置，显式选择不因缺少凭据而回退
fn one(root: &Value, provider: Provider, env: &dyn Fn(&str) -> Option<String>) -> Result<Service> {
    build(root, provider, env, true)
}

/// 【翻译服务】【表单预览】按 root、vendor 和 env 解析实际默认值及环境值，不要求凭据完整，返回服务参数
pub(crate) fn preview(
    root: &Value,
    vendor: &str,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Service> {
    let provider = match vendor {
        "bilibili" => Provider::Bilibili,
        "openai" => Provider::OpenAi,
        "responses" => Provider::Responses,
        "anthropic" => Provider::Anthropic,
        "gemini" => Provider::Gemini,
        "tencent" => Provider::Tencent,
        "baidu" => Provider::Baidu,
        "youdao" => Provider::Youdao,
        _ => return Err(Error::new(ErrorKind::Configuration, "external provider")),
    };
    build(root, provider, env, false)
}

/// 【翻译服务】【参数组装】按 root、provider、env 解析参数，validate 控制必填凭据校验，返回服务或错误
fn build(
    root: &Value,
    provider: Provider,
    env: &dyn Fn(&str) -> Option<String>,
    validate: bool,
) -> Result<Service> {
    let required = |value: &str, name: &str| {
        if validate {
            required(value, name)
        } else {
            Ok(())
        }
    };
    let vendor_key = match provider {
        Provider::Bilibili => "bilibili",
        Provider::Responses => "responses",
        Provider::OpenAi => "openai",
        Provider::Gemini => "gemini",
        Provider::Anthropic => "anthropic",
        Provider::Tencent => "tencent",
        Provider::Baidu => "baidu",
        Provider::Youdao => "youdao",
        _ => {
            return Err(Error::new(
                ErrorKind::Configuration,
                "external provider requires a process task",
            ));
        }
    };
    let vendor = root.get(vendor_key).unwrap_or(&Value::Null);
    // 1. 【翻译服务】【厂商隔离】Gemini 与 Anthropic 不继承公共 OpenAI 密钥、模型和提示词
    let shared = if matches!(
        provider,
        Provider::Gemini | Provider::Anthropic | Provider::Bilibili | Provider::Responses
    ) {
        &Value::Null
    } else {
        root
    };
    let get = |key: &str, names: &[&str], fallback: &str| {
        string(vendor, shared, key, env, names, fallback)
    };
    let timeout =
        |default| Duration::from_millis(integer(vendor, root, "timeoutMs", default, 1000, 300000));
    if matches!(
        provider,
        Provider::OpenAi
            | Provider::Gemini
            | Provider::Anthropic
            | Provider::Bilibili
            | Provider::Responses
    ) {
        let (endpoint, key, model) = match provider {
            Provider::Bilibili => (
                get("baseUrl", &[], "https://index-translate.bilibili.com/v1"),
                get("apiKey", &[], "empty"),
                get("model", &[], "Index-Translate-9B"),
            ),
            Provider::Responses => (
                get("baseUrl", &["OPENAI_BASE_URL"], "https://api.openai.com/v1"),
                get("apiKey", &["OPENAI_API_KEY"], ""),
                get("model", &["OPENAI_MODEL"], "gpt-5-mini"),
            ),
            Provider::OpenAi => {
                let key_env = get("apiKeyEnv", &[], "OPENAI_API_KEY");
                let base = get("apiBase", &[], "");
                (
                    if base.is_empty() {
                        get(
                            "baseUrl",
                            &[
                                "MARK_SHOT_LLM_API_BASE",
                                "OPENAI_BASE_URL",
                                "OPENAI_API_BASE",
                            ],
                            "https://api.openai.com/v1",
                        )
                    } else {
                        base
                    },
                    get("apiKey", &[&key_env, "MARK_SHOT_LLM_API_KEY"], ""),
                    get(
                        "model",
                        &["MARK_SHOT_LLM_MODEL", "OPENAI_MODEL"],
                        "gpt-4o-mini",
                    ),
                )
            }
            Provider::Gemini => (
                get(
                    "endpoint",
                    &["GEMINI_API_BASE", "MARK_SHOT_GEMINI_API_BASE"],
                    "https://generativelanguage.googleapis.com/v1beta",
                ),
                get(
                    "apiKey",
                    &["GEMINI_API_KEY", "MARK_SHOT_GEMINI_API_KEY"],
                    "",
                ),
                get(
                    "model",
                    &["GEMINI_MODEL", "MARK_SHOT_GEMINI_MODEL"],
                    "gemini-3.5-flash-lite",
                ),
            ),
            _ => (
                get(
                    "endpoint",
                    &["ANTHROPIC_API_BASE", "MARK_SHOT_ANTHROPIC_API_BASE"],
                    "https://api.anthropic.com/v1",
                ),
                get(
                    "apiKey",
                    &["ANTHROPIC_API_KEY", "MARK_SHOT_ANTHROPIC_API_KEY"],
                    "",
                ),
                get(
                    "model",
                    &["ANTHROPIC_MODEL", "MARK_SHOT_ANTHROPIC_MODEL"],
                    "claude-haiku-4-5",
                ),
            ),
        };
        required(&endpoint, "translation endpoint")?;
        required(&model, "translation model")?;
        required(&key, &format!("{} api key", provider.id()))?;
        let extra_body = super::request_options::parse_extra(
            field(vendor, "extraBody").or_else(|| field(shared, "extraBody")),
        )?;
        let flag = |key, default| {
            field(vendor, key)
                .or_else(|| field(shared, key))
                .and_then(Value::as_bool)
                .unwrap_or(default)
        };
        let temperature = field(vendor, "temperature")
            .and_then(Value::as_f64)
            .or_else(|| field(shared, "temperature").and_then(Value::as_f64))
            .or((provider == Provider::OpenAi).then_some(0.2));
        return Ok(Service::Llm(Llm {
            provider,
            endpoint,
            key: Secret(key),
            model,
            system_prompt: get("systemPrompt", &[], PROMPT),
            temperature,
            thinking_level: get("thinkingLevel", &[], ""),
            max_tokens: integer(vendor, shared, "maxTokens", 4096, 256, 8192),
            extra_body,
            filter_thinking: flag("filterThinking", true),
            disable_thinking: flag("disableThinking", provider == Provider::Bilibili),
            timeout: timeout(60000),
        }));
    }
    if provider == Provider::Tencent {
        let config = Tencent {
            endpoint: get("endpoint", &[], "tmt.tencentcloudapi.com"),
            secret_id: Secret(get(
                "secretId",
                &["TENCENTCLOUD_SECRET_ID", "MARK_SHOT_TENCENT_SECRET_ID"],
                "",
            )),
            secret_key: Secret(get(
                "secretKey",
                &["TENCENTCLOUD_SECRET_KEY", "MARK_SHOT_TENCENT_SECRET_KEY"],
                "",
            )),
            region: get(
                "region",
                &["TENCENTCLOUD_REGION", "MARK_SHOT_TENCENT_REGION"],
                "ap-guangzhou",
            ),
            project_id: integer(vendor, shared, "projectId", 0, 0, i32::MAX as u64),
            timeout: timeout(30000),
        };
        required(&config.secret_id.0, "tencent secretId")?;
        required(&config.secret_key.0, "tencent secretKey")?;
        required(&config.region, "tencent region")?;
        required(&config.endpoint, "tencent endpoint")?;
        return Ok(Service::Tencent(config));
    }
    let (app_id, key, endpoint) = if provider == Provider::Baidu {
        (
            get(
                "appId",
                &["MARK_SHOT_BAIDU_APP_ID", "BAIDU_TRANSLATE_APP_ID"],
                "",
            ),
            get(
                "appKey",
                &["MARK_SHOT_BAIDU_APP_KEY", "BAIDU_TRANSLATE_APP_KEY"],
                "",
            ),
            get(
                "endpoint",
                &[],
                "https://fanyi-api.baidu.com/api/trans/vip/translate",
            ),
        )
    } else {
        (
            get(
                "appKey",
                &["MARK_SHOT_YOUDAO_APP_KEY", "YOUDAO_APP_KEY"],
                "",
            ),
            get(
                "appSecret",
                &["MARK_SHOT_YOUDAO_APP_SECRET", "YOUDAO_APP_SECRET"],
                "",
            ),
            get("endpoint", &[], "https://openapi.youdao.com/api"),
        )
    };
    required(&app_id, &format!("{} application id", provider.id()))?;
    required(&key, &format!("{} application secret", provider.id()))?;
    required(&endpoint, "translation endpoint")?;
    Ok(Service::Form(FormService {
        provider,
        endpoint,
        app_id: Secret(app_id),
        key: Secret(key),
        timeout: timeout(30000),
    }))
}

/// 【翻译服务】【服务选择】按 root、preference 和 env 返回服务；auto 按原版固定顺序选首个配置完整的服务，不因请求失败换服务
pub(crate) fn service(
    root: &Value,
    preference: &str,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Service> {
    let preference = preference.trim().to_ascii_lowercase();
    let builtin = || {
        let mut shared = root.clone();
        if let Some(object) = shared.as_object_mut() {
            object.remove("openai");
        }
        one(&shared, Provider::OpenAi, env).map_err(|mut error| {
            error.provider = Some(Provider::OpenAi);
            error
        })
    };
    if preference == "builtin" {
        return builtin();
    }
    let id = preference
        .strip_prefix("plugin:")
        .unwrap_or_default()
        .trim();
    let selected = match id {
        "bilibili" => Some(Provider::Bilibili),
        "openai-responses" => Some(Provider::Responses),
        "openai-compatible" => Some(Provider::OpenAi),
        "gemini" => Some(Provider::Gemini),
        "anthropic" => Some(Provider::Anthropic),
        "tencent-tmt" => Some(Provider::Tencent),
        "baidu-fanyi" => Some(Provider::Baidu),
        "youdao-nmt" => Some(Provider::Youdao),
        _ => None,
    };
    if let Some(provider) = selected {
        return one(root, provider, env).map_err(|mut error| {
            error.provider = Some(provider);
            error
        });
    }
    if preference.starts_with("plugin:") {
        return Err(Error::new(
            ErrorKind::Configuration,
            format!("translation provider unavailable: {id}"),
        ));
    }
    // 2. 【翻译服务】【可用配置】仅在选择阶段跳过缺失配置，服务确定后保持原文只交给这一服务
    for provider in [
        Provider::OpenAi,
        Provider::Responses,
        Provider::Gemini,
        Provider::Anthropic,
        Provider::Tencent,
        Provider::Baidu,
        Provider::Youdao,
    ] {
        if let Ok(config) = one(root, provider, env) {
            return Ok(config);
        }
    }
    one(root, Provider::Bilibili, env)
}
