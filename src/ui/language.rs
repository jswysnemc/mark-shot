//! 【界面语言】【语言选择】沿用原版环境、配置及系统语言优先级，界面文案与业务文字保持独立

mod common;
mod ocr;
mod pinned_menu;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    System,
    English,
    Chinese,
}

impl Mode {
    /// 【界面语言】【配置解析】按 raw 识别原版配置别名，未知或空值返回跟随系统模式
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "english" | "en" | "en_us" | "en-us" => Self::English,
            "chinese" | "zh" | "zh_cn" | "zh-cn" | "cn" => Self::Chinese,
            _ => Self::System,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    English,
    Chinese,
}

impl Language {
    /// 【界面语言】【环境解析】按 raw 识别原版环境覆盖值及中文区域名，其他值返回英文
    fn from_environment(raw: &str) -> Self {
        let raw = raw.trim().to_ascii_lowercase();
        if raw.starts_with("zh") || matches!(raw.as_str(), "chinese" | "cn") {
            Self::Chinese
        } else {
            Self::English
        }
    }

    /// 【界面语言】【优先级选择】按 override_value、mode、locale 选择语言，非空环境覆盖值优先，返回有效语言
    pub fn resolve(override_value: Option<&str>, mode: Mode, locale: Option<&str>) -> Self {
        if let Some(value) = override_value.filter(|value| !value.trim().is_empty()) {
            return Self::from_environment(value);
        }
        match mode {
            Mode::English => Self::English,
            Mode::Chinese => Self::Chinese,
            Mode::System => {
                let locale = locale.unwrap_or_default().trim().to_ascii_lowercase();
                if locale == "zh" || locale.starts_with("zh_") || locale.starts_with("zh-") {
                    Self::Chinese
                } else {
                    Self::English
                }
            }
        }
    }

    /// 【界面语言】【当前配置】读取当前环境和独立配置并返回启动语言；非法配置记录错误后跟随系统，不改写配置
    pub fn current() -> Self {
        let mode = crate::config::interface_language::current().unwrap_or_else(|error| {
            log::error!("【界面语言】【配置读取】无法读取语言设置: {error}");
            Mode::System
        });
        Self::resolve(
            std::env::var("MARK_SHOT_LANG").ok().as_deref(),
            mode,
            sys_locale::get_locale().as_deref(),
        )
    }

    /// 【界面语言】【文案查询】按 source 返回原版中文或英文文案，未知字符串保持原样，不处理业务正文
    pub fn text(self, source: &str) -> &str {
        if self == Self::Chinese {
            common::lookup(source)
                .or_else(|| ocr::lookup(source))
                .or_else(|| pinned_menu::lookup(source))
                .unwrap_or(source)
        } else {
            source
        }
    }

    /// 【界面语言】【参数文案】按 source 和 values 替换原版编号占位符，返回完整文案；参数内容不参与翻译或再次替换
    pub fn format(self, source: &str, values: &[&str]) -> String {
        let source = self.text(source);
        let mut result = String::with_capacity(source.len());
        let mut remaining = source;
        while let Some(position) = remaining.find('%') {
            result.push_str(&remaining[..position]);
            remaining = &remaining[position..];
            let digits = remaining[1..]
                .bytes()
                .take_while(u8::is_ascii_digit)
                .count();
            let value = remaining[1..1 + digits]
                .parse::<usize>()
                .ok()
                .and_then(|index| index.checked_sub(1))
                .and_then(|index| values.get(index));
            if let Some(value) = value {
                result.push_str(value);
                remaining = &remaining[1 + digits..];
            } else {
                result.push('%');
                remaining = &remaining[1..];
            }
        }
        result.push_str(remaining);
        result
    }

    /// 【界面语言】【字形预载】返回当前界面所需字形样本；只用于启动时定向字体补充，英文不增加回退字体
    pub(crate) fn font_sample(self) -> &'static str {
        match self {
            Self::English => "",
            Self::Chinese => {
                "文字表格原图复制已失败撤销重做剪切粘贴删除全选取消编辑置顶图片关闭翻译目标语言快捷键更多操作识别结果设置保存中没有可始终开启收起调整窗口大小全文译字符行列超时服务提供程序未能启动未知没有翻译结果打开方式扩展找到面配置命令创建"
            }
        }
    }
}
