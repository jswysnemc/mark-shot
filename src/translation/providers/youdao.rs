//! 【有道翻译】【批次签名】多个 q 字段保持顺序，签名输入沿用原版 UTF-16 字符计数

use super::{
    super::{
        Error, ErrorKind, Output, Provider, Result, Segment,
        http::{Request, Transport},
        language,
        resolve::FormService,
        response,
    },
    batches, flattened, hex, random,
};
use serde_json::Value;

/// 【有道翻译】【签名输入】按 query 返回原版二十字符阈值的签名输入，超长取首尾十个 UTF-16 单元和完整长度
pub(super) fn sign_input(query: &str) -> String {
    let units: Vec<_> = query.encode_utf16().collect();
    if units.len() <= 20 {
        return query.to_owned();
    }
    format!(
        "{}{}{}",
        utf16_fragment(&units[..10]),
        units.len(),
        utf16_fragment(&units[units.len() - 10..])
    )
}

/// 【有道翻译】【截断片段】按 units 转为原版 UTF-8 片段，不完整代理对跳过，不插入替代字符
fn utf16_fragment(units: &[u16]) -> String {
    char::decode_utf16(units.iter().copied())
        .filter_map(std::result::Result::ok)
        .collect()
}

/// 【有道翻译】【签名摘要】按 app_key、query、salt、time 和 secret 返回 UTF-8 SHA-256 小写摘要，不使用分隔符
pub(super) fn signature(
    app_key: &str,
    query: &str,
    salt: &str,
    time: &str,
    secret: &str,
) -> String {
    hex(&hmac_sha256::Hash::hash(
        format!("{app_key}{}{salt}{time}{secret}", sign_input(query)).as_bytes(),
    ))
}

/// 【有道翻译】【表单编码】按 config、batch、code、salt 和 time 返回多个 q 字段，使用原版百分号编码，空格不改为加号
fn form(config: &FormService, batch: &[Segment], code: &str, salt: &str, time: &str) -> Vec<u8> {
    let joined = batch
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<String>();
    let sign = signature(&config.app_id.0, &joined, salt, time, &config.key.0);
    let pairs = batch
        .iter()
        .map(|segment| ("q", segment.text.as_str()))
        .chain([
            ("from", "auto"),
            ("to", code),
            ("appKey", &config.app_id.0),
            ("salt", salt),
            ("sign", &sign),
            ("signType", "v3"),
            ("curtime", time),
        ]);
    pairs
        .map(|(key, value)| format!("{key}={}", super::encode(value, false)))
        .collect::<Vec<_>>()
        .join("&")
        .into_bytes()
}

/// 【有道翻译】【服务回复】按 body 返回有序译文，数字或字符串零均成功，缺少错误码或译文数组均失败
fn parse(body: &Value) -> Result<Vec<String>> {
    let code = response::code(body.get("errorCode"));
    if code != "0" {
        let mut detail = format!(
            "youdao error {}",
            if code.is_empty() { "unknown" } else { &code }
        );
        let description = match code.as_str() {
            "101" => "missing required parameter",
            "102" => "unsupported language pair",
            "103" => "query text is too long",
            "105" => "unsupported signature type",
            "108" => "invalid application id",
            "111" => "invalid developer account",
            "113" => "query text is empty",
            "202" => "signature check failed",
            "203" => "client address is not in the allow list",
            "206" => "invalid timestamp",
            "207" => "replayed request",
            "401" => "account is in arrears",
            "411" => "request rate limited",
            "412" => "too many long requests",
            _ => "",
        };
        if !description.is_empty() {
            detail.push_str(&format!(": {description}"));
        }
        let hint = match code.as_str() {
            "202" => "appKey and appSecret look correct only if the query text is UTF-8 encoded",
            "206" => "curtime must be the current UTC timestamp in seconds, check the system clock",
            "207" => "salt and curtime must be regenerated for every request",
            _ => "",
        };
        if !hint.is_empty() {
            detail.push_str(&format!(" ({hint})"));
        }
        if let Some(id) = body
            .get("requestId")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
        {
            detail.push_str(&format!(" [requestId {id}]"));
        }
        return Err(Error::new(ErrorKind::Api, detail));
    }
    let rows = body
        .get("translation")
        .and_then(Value::as_array)
        .filter(|rows| !rows.is_empty())
        .ok_or_else(|| {
            Error::new(
                ErrorKind::Protocol,
                "youdao response missing translation array",
            )
        })?;
    Ok(rows
        .iter()
        .map(|value| value.as_str().unwrap_or_default().to_owned())
        .collect())
}

/// 【有道翻译】【完整批次】按 config、segments、target 和 http 逐批翻译，返回完整结果；每批生成新的随机 salt 与时间
pub(super) async fn translate(
    config: &FormService,
    segments: &[Segment],
    target: &str,
    http: &dyn Transport,
) -> Result<Output> {
    let code = language::code(Provider::Youdao, target)?;
    let pending = flattened(segments);
    let mut texts = Vec::new();
    for batch in batches(&pending, 4000) {
        let mut nonce = random()?;
        nonce[6] = (nonce[6] & 0x0f) | 0x40;
        nonce[8] = (nonce[8] & 0x3f) | 0x80;
        let value = hex(&nonce);
        let salt = format!(
            "{}-{}-{}-{}-{}",
            &value[..8],
            &value[8..12],
            &value[12..16],
            &value[16..20],
            &value[20..]
        );
        let time = chrono::Utc::now().timestamp().to_string();
        let body = http
            .send(Request {
                provider: Provider::Youdao,
                endpoint: config.endpoint.clone(),
                content_type: "application/x-www-form-urlencoded",
                headers: vec![],
                body: form(config, batch, code, &salt, &time),
                timeout: config.timeout,
            })
            .await?;
        let output = response::checked(Provider::Youdao, batch, parse(&body)?)?;
        texts.extend(output);
    }
    response::ordered(Provider::Youdao, segments, texts)
}
