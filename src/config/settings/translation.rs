//! 【翻译设置】【服务字段】声明各服务的独立字段与默认参数

pub(crate) struct Service {
    pub id: &'static str,
    pub vendor: &'static str,
    pub title: &'static str,
    pub address_key: &'static str,
}

pub(crate) const SERVICES: &[Service] = &[
    Service {
        id: "plugin:bilibili",
        vendor: "bilibili",
        title: "哔哩哔哩 / Bilibili",
        address_key: "baseUrl",
    },
    Service {
        id: "plugin:openai-compatible",
        vendor: "openai",
        title: "OpenAI Chat Completions",
        address_key: "apiBase",
    },
    Service {
        id: "plugin:openai-responses",
        vendor: "responses",
        title: "OpenAI Responses",
        address_key: "baseUrl",
    },
    Service {
        id: "plugin:anthropic",
        vendor: "anthropic",
        title: "Anthropic Messages",
        address_key: "endpoint",
    },
    Service {
        id: "plugin:gemini",
        vendor: "gemini",
        title: "Gemini",
        address_key: "endpoint",
    },
    Service {
        id: "plugin:tencent-tmt",
        vendor: "tencent",
        title: "腾讯机器翻译 / Tencent",
        address_key: "endpoint",
    },
    Service {
        id: "plugin:baidu-fanyi",
        vendor: "baidu",
        title: "百度翻译 / Baidu",
        address_key: "endpoint",
    },
    Service {
        id: "plugin:youdao-nmt",
        vendor: "youdao",
        title: "有道翻译 / Youdao",
        address_key: "endpoint",
    },
    Service {
        id: "command",
        vendor: "",
        title: "自定义命令 / Command",
        address_key: "",
    },
    Service {
        id: "helper",
        vendor: "",
        title: "外部程序 / Helper",
        address_key: "",
    },
];

impl Service {
    /// 【翻译设置】【服务卡片】按 self 与 chinese 返回紧凑名称和协议说明
    pub(crate) fn caption(&self, chinese: bool) -> (&'static str, &'static str) {
        match self.vendor {
            "bilibili" => (
                if chinese { "哔哩哔哩" } else { "Bilibili" },
                "Index-Translate",
            ),
            "openai" => ("OpenAI", "Chat Completions"),
            "responses" => ("OpenAI", "Responses API"),
            "anthropic" => ("Anthropic", "Messages API"),
            "gemini" => ("Gemini", "Google AI"),
            "tencent" => (
                if chinese { "腾讯翻译" } else { "Tencent" },
                if chinese {
                    "机器翻译"
                } else {
                    "Machine translation"
                },
            ),
            "baidu" => (
                if chinese { "百度翻译" } else { "Baidu" },
                if chinese {
                    "机器翻译"
                } else {
                    "Machine translation"
                },
            ),
            "youdao" => (
                if chinese { "有道翻译" } else { "Youdao" },
                if chinese {
                    "机器翻译"
                } else {
                    "Machine translation"
                },
            ),
            _ if self.id == "command" => (
                if chinese {
                    "自定义命令"
                } else {
                    "Command"
                },
                if chinese {
                    "本地命令模板"
                } else {
                    "Local command template"
                },
            ),
            _ => (
                if chinese { "外部程序" } else { "Helper" },
                if chinese {
                    "调用已配置的程序"
                } else {
                    "Configured helper"
                },
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Field {
    Provider,
    Target,
    Address,
    Key,
    Model,
    Models,
    Filter,
    Disable,
    Extra,
    Timeout,
    AppId,
    Region,
    Project,
    Command,
}

impl Field {
    /// 【翻译设置】【参数分组】按 self 和 chinese 返回顺序及分组标题
    pub(crate) fn group(self, chinese: bool) -> (u8, &'static str) {
        let (number, zh, en) = match self {
            Self::Provider | Self::Target => (0, "翻译偏好", "Translation preferences"),
            Self::Address
            | Self::Key
            | Self::AppId
            | Self::Region
            | Self::Project
            | Self::Command => (1, "连接配置", "Connection"),
            Self::Model | Self::Models | Self::Filter | Self::Disable => {
                (2, "模型与生成", "Model and generation")
            }
            Self::Extra | Self::Timeout => (3, "高级选项", "Advanced options"),
        };
        (number, if chinese { zh } else { en })
    }

    /// 【翻译设置】【紧凑文案】按 self 和 chinese 返回字段标题及独立帮助文字
    pub(crate) fn caption(self, chinese: bool) -> (&'static str, &'static str) {
        let (zh, en, help_zh, help_en) = match self {
            Self::Provider => (
                "翻译服务",
                "Service",
                "各服务独立保存配置",
                "Settings are saved per service",
            ),
            Self::Target => (
                "目标语言",
                "Target language",
                "默认翻译结果语言",
                "Default translation language",
            ),
            Self::Address => (
                "服务地址",
                "Base URL",
                "支持兼容接口与自定义地址",
                "Compatible or custom endpoint",
            ),
            Self::Key => (
                "接口密钥",
                "API key",
                "留空使用环境变量或默认值",
                "Empty: environment or default",
            ),
            Self::Model => (
                "模型编号",
                "Model ID",
                "手动填写或从目录选择",
                "Enter manually or use catalog",
            ),
            Self::Models => (
                "模型目录",
                "Model catalog",
                "查询当前服务可用模型",
                "Fetch models from this service",
            ),
            Self::Filter => (
                "过滤思考内容",
                "Filter thinking",
                "移除译文中的思考标签",
                "Remove thinking tags from output",
            ),
            Self::Disable => (
                "请求关闭思考",
                "Disable thinking",
                "需要模型支持对应参数",
                "Requires model support",
            ),
            Self::Extra => (
                "自定义请求体",
                "Custom body",
                "JSON 对象，支持粘贴多行",
                "JSON object; multiline paste",
            ),
            Self::Timeout => (
                "请求超时",
                "Timeout",
                "毫秒，1000–300000",
                "Milliseconds, 1000–300000",
            ),
            Self::AppId => (
                "应用编号",
                "App / Secret ID",
                "服务商提供的应用标识",
                "Application identifier",
            ),
            Self::Region => (
                "服务区域",
                "Region",
                "服务商区域编号",
                "Provider region identifier",
            ),
            Self::Project => (
                "项目编号",
                "Project ID",
                "服务商项目标识",
                "Provider project identifier",
            ),
            Self::Command => (
                "命令模板",
                "Command template",
                "使用已配置的本地翻译命令",
                "Configured local translator",
            ),
        };
        if chinese {
            (zh, help_zh)
        } else {
            (en, help_en)
        }
    }

    /// 【翻译设置】【字段名称】按 self 与 service 返回保存键，部分字段按厂商选择键名
    pub(crate) fn key(self, service: &Service) -> &'static str {
        match self {
            Self::Provider => "provider",
            Self::Target => "target_language",
            Self::Address => service.address_key,
            Self::Key => match service.vendor {
                "tencent" => "secretKey",
                "baidu" => "appKey",
                "youdao" => "appSecret",
                _ => "apiKey",
            },
            Self::AppId => match service.vendor {
                "tencent" => "secretId",
                "youdao" => "appKey",
                _ => "appId",
            },
            Self::Model => "model",
            Self::Filter => "filterThinking",
            Self::Disable => "disableThinking",
            Self::Extra => "extraBody",
            Self::Timeout => "timeoutMs",
            Self::Region => "region",
            Self::Project => "projectId",
            Self::Command => "command",
            Self::Models => "",
        }
    }

    /// 【翻译设置】【字段范围】无参数，返回字段是否直接保存在翻译配置根部
    pub fn global(self) -> bool {
        matches!(self, Self::Target | Self::Command)
    }

    /// 【翻译设置】【敏感标记】无参数，返回是否默认遮蔽字段显示
    pub fn secret(self) -> bool {
        matches!(self, Self::Key | Self::AppId)
    }
}

/// 【翻译设置】【表单顺序】按 service 返回该协议实际支持的配置字段，不显示无效模型选择
pub(crate) fn fields(service: &Service) -> Vec<Field> {
    use Field::*;
    let mut fields = vec![Provider, Target];
    if service.vendor.is_empty() {
        if service.id == "command" {
            fields.push(Command);
        }
        return fields;
    }
    fields.push(Address);
    if matches!(service.vendor, "tencent" | "baidu" | "youdao") {
        fields.push(AppId);
    }
    fields.push(Key);
    if matches!(
        service.vendor,
        "bilibili" | "openai" | "responses" | "anthropic" | "gemini"
    ) {
        fields.extend([Model, Models, Filter, Disable, Extra]);
    } else if service.vendor == "tencent" {
        fields.extend([Region, Project]);
    }
    fields.push(Timeout);
    fields
}
