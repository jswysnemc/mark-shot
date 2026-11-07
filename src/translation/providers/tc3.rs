//! 【腾讯翻译】【请求签名】保持规范请求与实际发送字节一致，三级密钥派生中间量始终为二进制

use super::{
    super::{Error, ErrorKind, Result},
    hex,
};

pub(super) const CONTENT_TYPE: &str = "application/json; charset=utf-8";

/// 【腾讯翻译】【规范请求】按 host、body 返回原版最小签名头集合的规范请求，不对请求正文重新编码
pub(super) fn canonical(host: &str, body: &[u8]) -> String {
    format!(
        "POST\n/\n\ncontent-type:{}\nhost:{}\n\ncontent-type;host\n{}",
        CONTENT_TYPE,
        host.trim().to_lowercase(),
        hex(&hmac_sha256::Hash::hash(body))
    )
}

/// 【腾讯翻译】【鉴权头】按 id、key、host、body 和 timestamp 返回 TC3 签名，UTC 日期参与凭证范围，非法时间返回错误
pub(super) fn authorization(
    id: &str,
    key: &str,
    host: &str,
    body: &[u8],
    timestamp: i64,
) -> Result<String> {
    let date = chrono::DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| Error::new(ErrorKind::Configuration, "invalid TC3 timestamp"))?
        .format("%Y-%m-%d")
        .to_string();
    let scope = format!("{date}/tmt/tc3_request");
    let to_sign = format!(
        "TC3-HMAC-SHA256\n{timestamp}\n{scope}\n{}",
        hex(&hmac_sha256::Hash::hash(canonical(host, body).as_bytes()))
    );
    // 1. 【腾讯翻译】【密钥派生】顺序为日期、服务、终止串，不能把中间结果转成十六进制
    let date_key = hmac_sha256::HMAC::mac(date.as_bytes(), format!("TC3{key}").as_bytes());
    let service_key = hmac_sha256::HMAC::mac(b"tmt", date_key);
    let signing_key = hmac_sha256::HMAC::mac(b"tc3_request", service_key);
    let signature = hex(&hmac_sha256::HMAC::mac(to_sign.as_bytes(), signing_key));
    Ok(format!(
        "TC3-HMAC-SHA256 Credential={id}/{scope}, SignedHeaders=content-type;host, Signature={signature}"
    ))
}
