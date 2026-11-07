//! 【腾讯翻译】【批量降级】批量接口下线后只调用单段接口，保持每批身份、签名和目标语言一致

use super::{
    super::{
        Error, ErrorKind, Output, Provider, Result, Segment,
        http::{Request, Transport},
        language,
        resolve::Tencent,
        response,
    },
    batches, flattened, tc3,
};
use serde_json::{Value, json};

/// 【腾讯翻译】【接入点】按 endpoint 返回仅含根路径的地址和实际 Host，兼容无协议主机及测试端口
fn endpoint(endpoint: &str) -> Result<(String, String)> {
    let value = if endpoint.trim().to_ascii_lowercase().starts_with("http://")
        || endpoint.trim().to_ascii_lowercase().starts_with("https://")
    {
        endpoint.trim().to_owned()
    } else {
        format!("https://{}", endpoint.trim())
    };
    let mut url = url::Url::parse(&value)
        .map_err(|_| Error::new(ErrorKind::Configuration, "invalid tencent endpoint"))?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host_str().is_none()
    {
        return Err(Error::new(
            ErrorKind::Configuration,
            "invalid tencent endpoint",
        ));
    }
    url.set_path("/");
    url.set_query(None);
    url.set_fragment(None);
    let host = url[url::Position::BeforeHost..url::Position::AfterPort].to_owned();
    Ok((url.to_string(), host))
}

/// 【腾讯翻译】【已签请求】按 config、action、payload 和 http 发送一次请求，返回 Response，时间戳在签名和请求头中共用
async fn post(
    config: &Tencent,
    action: &str,
    payload: Value,
    http: &dyn Transport,
) -> Result<Value> {
    let (endpoint, host) = endpoint(&config.endpoint)?;
    let body = payload.to_string().into_bytes();
    let timestamp = chrono::Utc::now().timestamp();
    let authorization = tc3::authorization(
        &config.secret_id.0,
        &config.secret_key.0,
        &host,
        &body,
        timestamp,
    )?;
    let body = http
        .send(Request {
            provider: Provider::Tencent,
            endpoint,
            content_type: tc3::CONTENT_TYPE,
            timeout: config.timeout,
            body,
            headers: vec![
                ("Host".into(), host),
                ("Authorization".into(), authorization),
                ("X-TC-Action".into(), action.into()),
                ("X-TC-Version".into(), "2018-03-21".into()),
                ("X-TC-Timestamp".into(), timestamp.to_string()),
                ("X-TC-Region".into(), config.region.clone()),
            ],
        })
        .await?;
    body.get("Response")
        .filter(|value| value.is_object())
        .cloned()
        .ok_or_else(|| {
            Error::new(
                ErrorKind::Protocol,
                "tencent response missing Response object",
            )
        })
}

/// 【腾讯翻译】【业务错误】按 body 返回业务错误，HTTP 成功不等同于翻译成功，错误包含服务请求身份
fn api_error(body: &Value) -> Option<Error> {
    let error = body.get("Error")?;
    Some(Error::new(
        ErrorKind::Api,
        format!(
            "tencent tmt error {}: {} (RequestId: {})",
            error
                .get("Code")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            error
                .get("Message")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            body.get("RequestId")
                .and_then(Value::as_str)
                .unwrap_or_default()
        ),
    ))
}

/// 【腾讯翻译】【单段降级】按 config、batch、code 和 http 逐段请求，返回有序译文，任一段失败终止整个任务
async fn individually(
    config: &Tencent,
    batch: &[Segment],
    code: &str,
    http: &dyn Transport,
) -> Result<Vec<String>> {
    let mut texts = Vec::new();
    for segment in batch {
        let body = post(config, "TextTranslate", json!({"SourceText":segment.text,"Source":"auto","Target":code,"ProjectId":config.project_id}), http).await?;
        if let Some(error) = api_error(&body) {
            return Err(error);
        }
        let text = body.get("TargetText").ok_or_else(|| {
            Error::new(ErrorKind::Protocol, "tencent response missing TargetText")
        })?;
        texts.push(text.as_str().unwrap_or_default().to_owned());
    }
    Ok(texts)
}

/// 【腾讯翻译】【完整批次】按 config、segments、target 和 http 请求，返回完整输出；下线或未知 Action 只触发一次降级
pub(super) async fn translate(
    config: &Tencent,
    segments: &[Segment],
    target: &str,
    http: &dyn Transport,
) -> Result<Output> {
    let code = language::code(Provider::Tencent, target)?;
    let pending = flattened(segments);
    let mut texts = Vec::new();
    let mut fallback = false;
    for batch in batches(&pending, 2000) {
        if fallback {
            texts.extend(individually(config, batch, code, http).await?);
            continue;
        }
        let body = post(
            config,
            "TextTranslateBatch",
            json!({"SourceTextList":batch.iter().map(|segment| &segment.text).collect::<Vec<_>>(),
            "Source":"auto","Target":code,"ProjectId":config.project_id}),
            http,
        )
        .await?;
        if let Some(error) = api_error(&body) {
            let code = body
                .pointer("/Error/Code")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_ascii_lowercase();
            if code.starts_with("actionoffline") || code.starts_with("invalidaction") {
                fallback = true;
                texts.extend(
                    individually(
                        config,
                        batch,
                        language::code(Provider::Tencent, target)?,
                        http,
                    )
                    .await?,
                );
                continue;
            }
            return Err(error);
        }
        let result = body
            .get("TargetTextList")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::Protocol,
                    "tencent response missing TargetTextList",
                )
            })?;
        let output = response::checked(
            Provider::Tencent,
            batch,
            result
                .iter()
                .map(|text| text.as_str().unwrap_or_default().to_owned())
                .collect(),
        )?;
        texts.extend(output);
    }
    response::ordered(Provider::Tencent, segments, texts)
}
