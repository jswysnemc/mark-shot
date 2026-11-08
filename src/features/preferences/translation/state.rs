//! 【翻译设置】【表单状态】读取兼容字段，保存独立服务参数，管理目录任务

use super::fields::{self, Field, SERVICES, Service};
use crate::{
    backend::window::WindowWake,
    config,
    translation::{models, request_options},
    ui::{fonts::UiFonts, single_line::SingleLine},
};
use serde_json::{Map, Value};
use std::path::Path;

pub(in crate::features::preferences) struct State {
    pub(super) root: Value,
    pub(super) selected: usize,
    pub(super) scroll: usize,
    pub(super) editing: Option<(Field, SingleLine)>,
    pub(super) reveal: bool,
    pub(super) focus: Option<super::layout::Hit>,
    pub(super) models: Vec<String>,
    pub(super) task: Option<models::Task>,
    pub(super) pressed: Option<super::layout::Hit>,
    pub(super) drag: Option<crate::ui::scrollbar::Drag>,
    pub(in crate::features::preferences) wake: Option<WindowWake>,
}

/// 【翻译设置】【兼容读取】按 object 与 key 返回驼峰或下划线字段，缺失返回空值
fn get<'a>(object: &'a Value, key: &str) -> &'a Value {
    object
        .get(key)
        .or_else(|| object.get(snake(key)))
        .unwrap_or(&Value::Null)
}

/// 【翻译设置】【兼容键名】按 key 生成下划线键名，返回转换结果
fn snake(key: &str) -> String {
    key.chars()
        .flat_map(|ch| {
            if ch.is_ascii_uppercase() {
                vec!['_', ch.to_ascii_lowercase()]
            } else {
                vec![ch]
            }
        })
        .collect()
}

impl State {
    /// 【翻译设置】【页面初始化】从 values 创建不启动网络的表单，返回状态，自动模式只显示解析后的服务标识
    pub fn new(values: &Value) -> Self {
        let root = values["translation"].clone();
        let mut provider = root["provider"].as_str().unwrap_or("auto").to_owned();
        if root["command"]
            .as_str()
            .is_some_and(|value| !value.trim().is_empty())
        {
            provider = "command".into();
        } else if (provider == "auto" || provider == "builtin")
            && let Ok(service) = crate::translation::resolve::service(&root, &provider, &|name| {
                std::env::var(name).ok()
            })
        {
            provider = format!("plugin:{}", service.provider().id());
        }
        Self {
            selected: SERVICES
                .iter()
                .position(|service| service.id == provider)
                .unwrap_or(0),
            root,
            scroll: 0,
            editing: None,
            reveal: false,
            focus: None,
            models: Vec::new(),
            task: None,
            pressed: None,
            drag: None,
            wake: None,
        }
    }

    /// 【翻译设置】【当前协议】无参数，返回当前选中服务的字段定义
    pub(super) fn service(&self) -> &'static Service {
        &SERVICES[self.selected]
    }

    /// 【翻译设置】【字段列表】无参数，返回当前服务的有序表单
    pub(super) fn fields(&self) -> Vec<Field> {
        fields::fields(self.service())
    }

    /// 【翻译设置】【字段取值】按 field 返回当前配置或默认值，凭据只从配置读取，不把环境密钥放入表单
    pub(super) fn value(&self, field: Field) -> Value {
        let service = self.service();
        if field == Field::Provider {
            return Value::from(service.title);
        }
        if !field.secret()
            && let Ok(preview) =
                crate::translation::resolve::preview(&self.root, service.vendor, &|name| {
                    std::env::var(name).ok()
                })
        {
            use crate::translation::resolve::Service;
            let resolved = match (field, preview) {
                (Field::Address, Service::Llm(value)) => Some(value.endpoint.into()),
                (Field::Address, Service::Tencent(value)) => Some(value.endpoint.into()),
                (Field::Address, Service::Form(value)) => Some(value.endpoint.into()),
                (Field::Model, Service::Llm(value)) => Some(value.model.into()),
                (Field::Filter, Service::Llm(value)) => Some(value.filter_thinking.into()),
                (Field::Disable, Service::Llm(value)) => Some(value.disable_thinking.into()),
                (Field::Timeout, Service::Llm(value)) => {
                    Some((value.timeout.as_millis() as u64).into())
                }
                (Field::Timeout, Service::Tencent(value)) => {
                    Some((value.timeout.as_millis() as u64).into())
                }
                (Field::Timeout, Service::Form(value)) => {
                    Some((value.timeout.as_millis() as u64).into())
                }
                _ => None,
            };
            if let Some(value) = resolved {
                return value;
            }
        }
        let key = field.key(service);
        let mut value = if field.global() {
            get(&self.root, key)
        } else {
            get(&self.root[service.vendor], key)
        };
        if value.is_null() && matches!(service.vendor, "openai" | "tencent" | "baidu" | "youdao") {
            value = get(&self.root, key);
        }
        if value.is_null() && field == Field::Timeout {
            value = get(&self.root, key);
        }
        if field == Field::Address && value.is_null() && service.vendor == "openai" {
            value = get(&self.root[service.vendor], "baseUrl");
            if value.is_null() {
                value = get(&self.root, "baseUrl");
            }
        }
        if !value.is_null() {
            return value.clone();
        }
        match field {
            Field::Target => "Simplified Chinese".into(),
            Field::Address => "".into(),
            Field::Model => "".into(),
            Field::Filter => true.into(),
            Field::Disable => (service.vendor == "bilibili").into(),
            Field::Extra => "{}".into(),
            Field::Timeout => 60000.into(),
            Field::Region => "ap-guangzhou".into(),
            Field::Project => 0.into(),
            _ => "".into(),
        }
    }

    /// 【翻译设置】【编辑开始】按 field 与 fonts 创建字段编辑器，返回空值，默认选中全文并遮蔽敏感字段
    pub(super) fn edit(&mut self, field: Field, fonts: &mut UiFonts) {
        let value = self.value(field);
        let text = value
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| value.to_string());
        let mut editor = SingleLine::new(
            &text,
            fonts,
            if field == Field::Extra { 65536 } else { 8192 },
        );
        editor.select_all(fonts);
        self.editing = Some((field, editor));
        self.reveal = false;
    }

    /// 【翻译设置】【事务提交】按 path 和 patch 保存配置并刷新运行时，返回错误文本；不输出凭据
    fn persist(&mut self, path: &Path, patch: Map<String, Value>) -> Result<(), String> {
        config::translation::preferences::save(path, &patch).map_err(|_| {
            "保存失败，请检查配置格式和文件权限 / Unable to save configuration".to_owned()
        })?;
        if !cfg!(test) {
            config::preferences::reload(path)
                .map_err(|_| "重新读取配置失败 / Unable to reload configuration".to_owned())?;
        }
        self.root = config::preferences::read(path).map_err(|_| "重新读取配置失败 / Unable to reload configuration".to_owned())?["translation"].clone();
        Ok(())
    }

    /// 【翻译设置】【字段保存】按 path、field 和 text 校验并保存单字段，返回结果，非法 JSON 保留编辑状态
    pub(super) fn save(&mut self, path: &Path, field: Field, text: &str) -> Result<(), String> {
        let value = match field {
            Field::Extra => {
                request_options::parse_extra(Some(&Value::from(text)))
                    .map_err(|error| error.to_string())?;
                Value::from(text)
            }
            Field::Timeout | Field::Project => {
                let number: u64 = text
                    .trim()
                    .parse()
                    .map_err(|_| "请输入整数 / Enter an integer".to_owned())?;
                if (field == Field::Timeout && !(1000..=300000).contains(&number))
                    || number > i32::MAX as u64
                {
                    return Err("数值超出范围 / Value is out of range".into());
                }
                number.into()
            }
            Field::Filter | Field::Disable => (text == "true").into(),
            Field::Address => {
                let address = text.trim();
                let url = if self.service().vendor == "tencent" && !address.contains("://") {
                    format!("https://{address}")
                } else {
                    address.to_owned()
                };
                let valid = url::Url::parse(&url).is_ok_and(|url| {
                    matches!(url.scheme(), "http" | "https")
                        && url.host_str().is_some()
                        && url.username().is_empty()
                        && url.password().is_none()
                        && url.query().is_none()
                        && url.fragment().is_none()
                });
                if !valid {
                    return Err("请输入有效服务地址，不含凭据或查询参数 / Invalid endpoint".into());
                }
                address.into()
            }
            _ => text.trim().into(),
        };
        let service = self.service();
        let key = field.key(service);
        let mut patch = Map::new();
        if field.global() {
            patch.insert(key.into(), value);
        } else {
            let mut vendor = Map::new();
            vendor.insert(key.into(), value);
            patch.insert(service.vendor.into(), vendor.into());
        }
        self.persist(path, patch)?;
        self.editing = None;
        self.reveal = false;
        if matches!(field, Field::Address | Field::Key) {
            self.task = None;
            self.models.clear();
        }
        Ok(())
    }

    /// 【翻译设置】【服务查看】按 index 切换查看服务，path 保持接口兼容，不修改已启用服务，返回结果
    pub(in crate::features::preferences) fn select(
        &mut self,
        path: &Path,
        index: usize,
    ) -> Result<(), String> {
        let _ = path;
        self.selected = index;
        self.cancel();
        self.scroll = 0;
        self.models.clear();
        Ok(())
    }

    /// 【翻译设置】【启用服务】按 path 保存当前服务或 auto 自动选择，显式启用时清除命令覆盖，返回结果
    pub(super) fn enable(&mut self, path: &Path, auto: bool) -> Result<(), String> {
        let mut patch = Map::new();
        let provider = if auto { "auto" } else { self.service().id };
        patch.insert("provider".into(), provider.into());
        if provider != "command" {
            patch.insert("command".into(), "".into());
        }
        self.persist(path, patch)
    }

    /// 【翻译设置】【目录查询】无参数，按当前表单快照启动服务 models 查询，返回启动结果
    pub(super) fn query(&mut self) -> Result<(), String> {
        let wake = self.wake.clone();
        self.task = Some(
            models::Task::start(&self.root, self.service().id, move || {
                if let Some(wake) = wake {
                    wake.notify();
                }
            })
            .map_err(|error| error.to_string())?,
        );
        Ok(())
    }

    /// 【翻译设置】【交互取消】无参数，取消尚未保存编辑、菜单和查询，返回空值
    pub(in crate::features::preferences) fn cancel(&mut self) {
        self.task = None;
        self.editing = None;
        self.reveal = false;
        self.pressed = None;
        self.drag = None;
    }
}
