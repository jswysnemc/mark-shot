//! 【KDE窗口识别】【候选校验】将单次 KWin 记录转换为可见窗口的全局逻辑边界

use super::super::{Result, Window, geometry};
use serde::Deserialize;

pub(super) const MAX_REPLY: usize = 1024 * 1024;

#[derive(Deserialize)]
struct Record {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    z: i32,
    eligible: bool,
    minimized: bool,
    hidden: bool,
    deleted: bool,
    on_desktop: bool,
    on_activity: bool,
}

/// 【KDE窗口识别】【快照解码】按 reply 校验有界记录并生成候选，返回窗口列表或协议错误
pub(super) fn decode(reply: &str) -> Result<Vec<Window>> {
    if reply.len() > MAX_REPLY {
        return Err("KWin sent an oversized window snapshot".into());
    }
    let records: Vec<Record> = serde_json::from_str(reply)?;
    if records.len() > 4096 {
        return Err("KWin sent too many windows".into());
    }
    let mut windows = Vec::new();
    for record in records {
        if record.eligible
            && !record.minimized
            && !record.hidden
            && !record.deleted
            && record.on_desktop
            && record.on_activity
            && let Some(rect) = geometry::rect(record.x, record.y, record.width, record.height)
        {
            geometry::append(&mut windows, rect, Some(record.z));
        }
    }
    Ok(windows)
}
