//! 【截图历史】【文件身份】严格文件标识、UTC 时间与唯一名称

use std::io;

use chrono::{DateTime, NaiveDateTime, Utc};

/// 【截图历史】【身份解析】检查 name 的固定格式和合法日期；返回 UTC 时间，任意路径返回空值
pub fn timestamp(name: &str) -> Option<DateTime<Utc>> {
    let bytes = name.as_bytes();
    if bytes.len() != 62
        || !name.starts_with("capture-")
        || bytes[25] != b'-'
        || !name.ends_with(".png")
        || !bytes[8..25].iter().all(u8::is_ascii_digit)
        || !bytes[26..58]
            .iter()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return None;
    }
    let time = NaiveDateTime::parse_from_str(&name[8..25], "%Y%m%d%H%M%S%3f").ok()?;
    if chrono::Datelike::year(&time) < 1 || chrono::Timelike::nanosecond(&time) >= 1_000_000_000 {
        return None;
    }
    Some(time.and_utc())
}

/// 【截图历史】【名称生成】用 now 和 previous 生成严格递增 UTC 时间及随机标识；返回历史文件名
pub(crate) fn next(now: DateTime<Utc>, previous: Option<DateTime<Utc>>) -> io::Result<String> {
    // 1. 【截图历史】【毫秒排序】先量化当前时间，再比较文件名中的毫秒，防止同毫秒随机顺序
    let now = DateTime::from_timestamp_millis(now.timestamp_millis())
        .ok_or_else(|| io::Error::other("history timestamp is out of range"))?;
    let stamp = if let Some(previous) = previous.filter(|previous| *previous >= now) {
        previous
            .checked_add_signed(chrono::Duration::milliseconds(1))
            .ok_or_else(|| io::Error::other("history timestamp overflow"))?
    } else {
        now
    };
    if !(1..=9999).contains(&chrono::Datelike::year(&stamp)) {
        return Err(io::Error::other("history timestamp is out of range"));
    }
    let mut random = [0_u8; 16];
    getrandom::fill(&mut random).map_err(|error| io::Error::other(error.to_string()))?;
    random[6] = random[6] & 0x0f | 0x40;
    random[8] = random[8] & 0x3f | 0x80;
    let id: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!(
        "capture-{}-{id}.png",
        stamp.format("%Y%m%d%H%M%S%3f")
    ))
}
