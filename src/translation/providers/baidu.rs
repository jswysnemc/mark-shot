//! 【百度翻译】【表单请求】在编码前签名原文，批次条数不符时拒绝整批回填

use super::{
    super::{
        Error, ErrorKind, Output, Provider, Result, Segment,
        http::{Request, Transport},
        language,
        resolve::FormService,
        response,
    },
    batches, flattened, random,
};
use md5::{Digest, Md5};
use serde_json::Value;

/// 【百度翻译】【原文签名】按 app_id、query、salt 和 key 返回 UTF-8 MD5 小写摘要，不添加分隔符
pub(super) fn signature(app_id: &str, query: &str, salt: &str, key: &str) -> String {
    format!(
        "{:x}",
        Md5::digest(format!("{app_id}{query}{salt}{key}").as_bytes())
    )
}

/// 【百度翻译】【表单编码】按 config、query、code 和 salt 返回原版顺序的表单，空格使用加号，原始加号和百分号必须编码
fn form(config: &FormService, query: &str, code: &str, salt: &str) -> Vec<u8> {
    [
        ("q", query),
        ("from", "auto"),
        ("to", code),
        ("appid", &config.app_id.0),
        ("salt", salt),
        (
            "sign",
            &signature(&config.app_id.0, query, salt, &config.key.0),
        ),
    ]
    .into_iter()
    .map(|(key, value)| format!("{key}={}", super::encode(value, true)))
    .collect::<Vec<_>>()
    .join("&")
    .into_bytes()
}

/// 【百度翻译】【服务回复】按 body 返回有序译文，支持字符串或整数错误码，业务失败不返回部分译文
fn parse(body: &Value) -> Result<Vec<String>> {
    if let Some(rows) = body
        .get("trans_result")
        .and_then(Value::as_array)
        .filter(|rows| !rows.is_empty())
    {
        return Ok(rows
            .iter()
            .map(|row| {
                row.get("dst")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect());
    }
    let code = response::code(body.get("error_code"));
    if !code.is_empty() {
        let mut detail = format!("baidu translate error {code}");
        let message = response::code(body.get("error_msg"));
        if !message.is_empty() {
            detail.push_str(&format!(": {message}"));
        }
        let description = match code.as_str() {
            "52001" => "请求超时，可减少单次文本量后重试",
            "52002" => "系统错误",
            "52003" => "未授权用户，确认 appId 与服务开通状态",
            "54000" => "必填参数为空",
            "54001" => "签名错误",
            "54003" => "访问频率受限",
            "54004" => "账户余额不足",
            "54005" => "长 query 请求频繁",
            "58000" => "客户端 IP 非法",
            "58001" => "译文语言方向不支持",
            "58002" => "服务已关闭",
            "90107" => "认证未通过或未生效",
            _ => "",
        };
        if !description.is_empty() {
            detail.push_str(&format!(" ({description})"));
        }
        if code == "54001" {
            detail.push_str("；检查 appId/appKey 与签名拼接顺序");
        }
        return Err(Error::new(ErrorKind::Api, detail));
    }
    Err(Error::new(
        ErrorKind::Protocol,
        "baidu response missing trans_result",
    ))
}

/// 【百度翻译】【完整批次】按 config、segments、target 和 http 顺序请求，返回完整输出；任一批失败则没有部分结果
pub(super) async fn translate(
    config: &FormService,
    segments: &[Segment],
    target: &str,
    http: &dyn Transport,
) -> Result<Output> {
    let code = language::code(Provider::Baidu, target)?;
    let pending: Vec<_> = flattened(segments)
        .into_iter()
        .filter(|segment| !segment.text.is_empty())
        .collect();
    let mut translated = std::collections::BTreeMap::new();
    for batch in batches(&pending, 1800) {
        let query = batch
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let salt = u32::from_ne_bytes(random()?[..4].try_into().unwrap()).to_string();
        let result = http
            .send(Request {
                provider: Provider::Baidu,
                endpoint: config.endpoint.clone(),
                content_type: "application/x-www-form-urlencoded",
                headers: vec![],
                body: form(config, &query, code, &salt),
                timeout: config.timeout,
            })
            .await?;
        let output = response::checked(Provider::Baidu, batch, parse(&result)?)?;
        translated.extend(
            batch
                .iter()
                .zip(output)
                .map(|(segment, text)| (segment.id, text)),
        );
    }
    response::mapped(Provider::Baidu, segments, &translated)
}
