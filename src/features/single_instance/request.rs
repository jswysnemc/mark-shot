//! 【单实例】【命令格式】保留原版截图 JSON 字段和有界回复，录制命令明确拒绝

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub const MAX_PACKET: usize = 16 * 1024;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Request {
    pub capture: bool,
    pub fullscreen: bool,
    pub all_outputs: bool,
    pub capture_delay_ms: i64,
    pub recording_status: bool,
    pub stop_recording: bool,
    pub pause_recording: bool,
    pub resume_recording: bool,
    pub toggle_pause_recording: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    pub fullscreen: bool,
    pub all_outputs: bool,
    pub deadline: Option<Instant>,
}

#[derive(Debug, Deserialize)]
pub struct Response {
    pub handled: bool,
    pub message: String,
}

impl Request {
    /// 【单实例】【截图计划】按 self 和当前 now 返回截图参数与期限；负延迟按原版归零，超限或录制请求返回错误
    pub fn plan(&self, now: Instant) -> Result<Plan, &'static str> {
        if self.recording_status
            || self.stop_recording
            || self.pause_recording
            || self.resume_recording
            || self.toggle_pause_recording
        {
            return Err("recording commands are unavailable in this build");
        }
        if self.capture_delay_ms > 60_000 {
            return Err("capture delay must not exceed 60 seconds");
        }
        let delay = self.capture_delay_ms.max(0) as u64;
        let deadline = if delay == 0 {
            None
        } else {
            Some(
                now.checked_add(Duration::from_millis(delay))
                    .ok_or("capture delay overflows the clock")?,
            )
        };
        Ok(Plan {
            fullscreen: self.fullscreen,
            all_outputs: self.all_outputs,
            deadline,
        })
    }

    /// 【单实例】【命令编码】按 self 返回原版换行 JSON 请求，序列化失败返回错误，不包含配置或图片正文
    pub fn encode(&self) -> Result<Vec<u8>, serde_json::Error> {
        let mut bytes = serde_json::to_vec(self)?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

/// 【单实例】【回复编码】按 handled 和 message 返回换行 JSON；录制状态始终明确为空闲，不接受录制控制
pub fn response(handled: bool, message: &str) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&serde_json::json!({
        "handled": handled,
        "message": message,
        "stopped": false,
        "paused": false,
        "recording": {
            "active": false,
            "paused": false,
            "mode": "gif",
            "fps": 0,
            "frameCount": 0,
            "elapsedMs": 0,
            "outputPath": "",
        },
    }))
    .expect("fixed single-instance response must be serializable");
    bytes.push(b'\n');
    bytes
}
