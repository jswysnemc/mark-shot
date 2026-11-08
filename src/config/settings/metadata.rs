//! 【应用设置】【来源说明】配置来源、环境覆盖和控件约束共用定义
use super::{Entry, Kind};
use serde_json::Value;

impl Entry {
    /// 【应用设置】【字段说明】按 source、chinese 返回实际来源和帮助，避免把继承值误标为已保存值
    pub fn note(&self, source: &Value, chinese: bool) -> String {
        if matches!(self.kind, Kind::Link(_)) {
            return self.help(chinese).into();
        }
        let path = self.path();
        if super::super::runtime_overrides::contains(&path) {
            return if chinese {
                "当前进程受命令行参数覆盖；修改保存到下次启动"
            } else {
                "Command-line override active; edits apply next launch"
            }
            .into();
        }
        let environment = match path.as_str() {
            "ui.language" => Some("MARK_SHOT_LANG"),
            "log.level" => Some("MARK_SHOT_LOG"),
            "upload.command" => Some("MARK_SHOT_UPLOAD_COMMAND"),
            "upload.timeout_ms" => Some("MARK_SHOT_UPLOAD_TIMEOUT_MS"),
            _ => None,
        };
        if let Some(name) = environment
            .filter(|name| std::env::var(name).is_ok_and(|value| !value.trim().is_empty()))
        {
            return format!(
                "{}: {name}",
                if chinese {
                    "环境变量覆盖"
                } else {
                    "Environment override"
                }
            );
        }
        let help = self.help(chinese);
        if !help.is_empty() {
            return help.into();
        }
        let source = if super::super::settings_store::explicit(source, &path) {
            if chinese {
                "配置文件"
            } else {
                "Configuration file"
            }
        } else if chinese {
            "继承默认值"
        } else {
            "Inherited default"
        };
        if let Some((minimum, maximum)) = self.range() {
            format!("{source} · {minimum}–{maximum}")
        } else {
            source.into()
        }
    }
}
