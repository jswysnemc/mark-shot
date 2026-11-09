//! 【应用设置】【统一定义】集中字段类型、默认值、输入校验与生效时机，界面和写入端共用

mod catalog;
mod desktop;
mod extra;
mod metadata;
mod shortcuts;
pub(crate) mod translation;
pub use catalog::{Entry, Kind, Page, value_label};
use serde_json::Value;
use std::{io, sync::LazyLock};

pub static ENTRIES: LazyLock<Vec<Entry>> = LazyLock::new(|| {
    let mut entries: Vec<_> = catalog::BASE
        .iter()
        .filter(|entry| entry.section != "keys")
        .cloned()
        .collect();
    entries.extend_from_slice(extra::ENTRIES);
    entries.extend(shortcuts::entries());
    entries.extend_from_slice(desktop::ENTRIES);
    entries.extend(translation::SERVICES.iter().map(|service| Entry {
        page: Page::Translation,
        section: "translation",
        key: service.vendor,
        labels: (service.title, service.title),
        kind: Kind::Link(Page::Translation),
    }));
    entries
});
static DEFAULTS: LazyLock<Value> =
    LazyLock::new(|| serde_json::to_value(super::Config::default()).unwrap());

impl Entry {
    /// 【应用设置】【候选文案】按 value 和 chinese 返回具有字段语义的选项名称，原始值用于保存
    pub fn option_label<'a>(&self, value: &'a str, chinese: bool) -> &'a str {
        match (self.section, self.key, value) {
            ("general", "to", "") => {
                if chinese {
                    "不执行完成操作"
                } else {
                    "No output action"
                }
            }
            ("ocr", "engine", "system") => {
                if chinese {
                    "Windows 系统 OCR"
                } else {
                    "Windows OCR"
                }
            }
            _ => value_label(value, chinese),
        }
    }
    /// 【应用设置】【存储路径】无参数，返回分组和字段组成的规范路径
    pub fn path(&self) -> String {
        format!("{}.{}", self.section, self.key)
    }

    /// 【应用设置】【默认值】按配置结构及动作表返回字段默认值，空值保留继承语义
    pub fn default_value(&self) -> Value {
        match (self.section, self.key) {
            ("keys", key) => crate::keys::ACTIONS
                .iter()
                .find(|(name, _, _)| *name == key)
                .map(|(_, _, keys)| serde_json::json!(keys))
                .unwrap_or(Value::Null),
            ("log", "level") => "info".into(),
            ("startup", "selection_loupe") => true.into(),
            ("ui", "language") => "system".into(),
            _ => super::settings_store::value(&DEFAULTS, &self.path()).clone(),
        }
    }

    /// 【应用设置】【显示值】从 values 读取有效字段，缺失时使用结构默认值，返回可编辑文本
    pub fn display(&self, values: &Value) -> String {
        if matches!(self.kind, Kind::Link(_)) {
            return String::new();
        }
        let mut value = super::settings_store::value(values, &self.path());
        if self.section == "ui"
            && self.key == "language"
            && (value.is_null() || value.as_str() == Some(""))
        {
            value = &values["language"];
        }
        let value = if value.is_null() {
            self.default_value()
        } else {
            value.clone()
        };
        match value {
            Value::Null => String::new(),
            Value::String(text)
                if self.section == "ui" && self.key == "language" && text.is_empty() =>
            {
                "system".into()
            }
            Value::String(text) => text,
            Value::Array(values) if matches!(self.kind, Kind::Shortcut) => values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" / "),
            value => value.to_string(),
        }
    }

    /// 【应用设置】【输入解码】按字段类型将 text 转为配置值，返回范围或格式错误，不写文件
    pub fn parse(&self, text: &str) -> io::Result<Value> {
        if text.is_empty() && self.default_value().is_null() {
            return Ok(Value::Null);
        }
        let value = if self.section == "upload" && self.key == "env" {
            serde_json::from_str(text).map_err(io::Error::other)?
        } else if self.default_value().is_number() {
            serde_json::from_str::<Value>(text).map_err(io::Error::other)?
        } else {
            Value::String(text.into())
        };
        self.validate(&value)?;
        Ok(value)
    }

    /// 【应用设置】【字段约束】按 value 检查注册类型及范围，恢复继承允许空值，失败返回错误
    pub fn validate(&self, value: &Value) -> io::Result<()> {
        if value.is_null() {
            return Ok(());
        }
        let valid = match self.kind {
            Kind::Toggle => value.is_boolean(),
            Kind::Choice(choices) | Kind::Segments(choices) => {
                value.as_str().is_some_and(|text| choices.contains(&text))
            }
            Kind::Shortcut => value.is_string() || value.is_array(),
            Kind::Color => value
                .as_str()
                .is_some_and(|text| crate::theme::Rgba::parse_hex(text).is_some()),
            Kind::Link(_) => false,
            Kind::Text => match self.default_value() {
                Value::Number(default) => value.as_f64().is_some_and(|number| {
                    number.is_finite()
                        && (!default.is_i64() && !default.is_u64() || number.fract() == 0.0)
                        && self
                            .range()
                            .is_none_or(|(minimum, maximum)| number >= minimum && number <= maximum)
                }),
                Value::Object(_) => value
                    .as_object()
                    .is_some_and(|fields| fields.values().all(Value::is_string)),
                _ => value.is_string(),
            },
        };
        if !valid {
            return Err(io::Error::other(format!(
                "{}: invalid type, option or range / 类型、选项或范围无效",
                self.path()
            )));
        }
        if matches!(self.key, "border_color" | "default_color")
            && !value
                .as_str()
                .is_some_and(|text| crate::theme::Rgba::parse_hex(text).is_some())
        {
            return Err(io::Error::other("颜色格式应为 #RRGGBB 或 #RRGGBBAA"));
        }
        Ok(())
    }

    /// 【应用设置】【数值范围】无参数，返回与业务约束一致的闭区间，没有限制时返回空值
    pub fn range(&self) -> Option<(f64, f64)> {
        match (self.section, self.key) {
            ("history", "limit") => Some((1.0, 200.0)),
            ("history", "max_size_mib") => Some((1.0, 4096.0)),
            ("pinned", "border_width") => Some((0.0, 16.0)),
            ("magnifier", "zoom") => Some((2.0, 12.0)),
            ("export.image_frame", "shadow_opacity") => Some((0.0, 1.0)),
            ("export.image_frame", "padding") => Some((0.0, 256.0)),
            ("export.image_frame", _) => Some((0.0, 128.0)),
            ("upload", "timeout_ms") => Some((1000.0, i32::MAX as f64)),
            _ => None,
        }
    }

    /// 【应用设置】【数值步长】无参数，返回数值控件的步长，非数值字段返回空值
    pub fn step(&self) -> Option<f64> {
        if !self.default_value().is_number() {
            return None;
        }
        Some(match (self.section, self.key) {
            ("export.image_frame", "shadow_opacity") => 0.01,
            ("pinned", "border_width") => 0.5,
            ("upload", "timeout_ms") => 1000.0,
            _ => 1.0,
        })
    }

    /// 【应用设置】【生效说明】按 chinese 返回保存后生效阶段，界面提示与注册字段共用
    pub fn effect(&self, chinese: bool) -> &'static str {
        let labels = match (self.section, self.key) {
            ("desktop", _) => (
                "已保存，正在更新桌面注册",
                "Saved; updating desktop registration",
            ),
            ("ui" | "keys", _) => ("已保存，立即生效", "Saved; applies immediately"),
            ("log", _) => ("已保存，下次启动生效", "Saved; applies next launch"),
            ("general", "to") | ("startup", "selection_loupe" | "normal_tool") => (
                "已保存，关闭设置后生效",
                "Saved; applies when settings close",
            ),
            (
                "general" | "ocr" | "startup" | "magnifier" | "region_detection"
                | "window_detection",
                _,
            ) => ("已保存，下次截图生效", "Saved; applies next capture"),
            ("toolbar_layout", _) => (
                "已保存，关闭设置后更新工具栏",
                "Saved; toolbar updates when settings close",
            ),
            _ => (
                "已保存，下次执行对应操作时生效",
                "Saved; applies to the next operation",
            ),
        };
        if chinese { labels.0 } else { labels.1 }
    }

    /// 【应用设置】【检索文本】无参数，组合双语文案、说明、候选值和服务字段，返回可全文匹配文字
    pub fn search_text(&self) -> String {
        let mut text = format!(
            "{} {} {} {} {} {} {}",
            self.labels.0,
            self.labels.1,
            self.path(),
            self.help(true),
            self.help(false),
            self.page.title(true),
            self.page.title(false)
        );
        if let Kind::Choice(choices) | Kind::Segments(choices) = self.kind {
            for choice in choices {
                text.push_str(&format!(" {choice} {}", value_label(choice, true)));
            }
        }
        if let Kind::Link(Page::Translation) = self.kind
            && let Some(service) = translation::SERVICES
                .iter()
                .find(|service| service.vendor == self.key)
        {
            for field in translation::fields(service) {
                text.push_str(&format!(
                    " {} {} {} {}",
                    field.caption(true).0,
                    field.caption(true).1,
                    field.caption(false).0,
                    field.caption(false).1
                ));
            }
        }
        text
    }
}
