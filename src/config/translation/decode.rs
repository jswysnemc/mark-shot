//! 【翻译配置】【字段存在性】区分缺省任务参数与用户显式服务参数，不把默认六十秒注入厂商配置

use super::Translation;
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

#[derive(Deserialize)]
#[serde(default)]
struct Fields {
    #[serde(alias = "targetLanguage")]
    target_language: String,
    provider: String,
    command: String,
    #[serde(alias = "timeoutMs", deserialize_with = "timeout")]
    timeout_ms: Option<u64>,
    #[serde(flatten)]
    settings: Map<String, Value>,
}

impl Default for Fields {
    /// 【翻译配置】【缺省字段】返回没有显式超时的字段集合，无参数，沿用窗口目标语言和服务默认值
    fn default() -> Self {
        let defaults = Translation::default();
        Self {
            target_language: defaults.target_language,
            provider: defaults.provider,
            command: defaults.command,
            timeout_ms: None,
            settings: Map::new(),
        }
    }
}

/// 【翻译配置】【显式超时】按 deserializer 读取非空无符号整数，返回存在性；null 或错误类型不能伪装成缺省字段
fn timeout<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u64>, D::Error> {
    u64::deserialize(deserializer).map(Some)
}

impl<'de> Deserialize<'de> for Translation {
    /// 【翻译配置】【完整解码】按 deserializer 返回稳定窗口参数与原始厂商配置，记录用户是否显式设置全局超时
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let fields = Fields::deserialize(deserializer)?;
        Ok(Self {
            target_language: fields.target_language,
            provider: fields.provider,
            command: fields.command,
            timeout_ms: fields.timeout_ms.unwrap_or(60000),
            timeout_explicit: fields.timeout_ms.is_some(),
            settings: fields.settings,
        })
    }
}
