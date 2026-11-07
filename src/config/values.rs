use super::*;

// Value types
// ==========================================

/// color that defaults to a theme role unless explicitly set in config (`""` also keeps the default).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ThemeColor {
    explicit: Option<Rgba>,
    fallback: Option<Rgba>,
    role: Option<crate::theme::Role>,
}

impl ThemeColor {
    /// 【主题配置】【颜色读取】返回显式颜色或当前继承角色，无其他参数；未解析配置使用可见的错误颜色
    pub fn get(self) -> Rgba {
        self.explicit.unwrap_or_else(|| {
            let fallback = self.fallback.unwrap_or(Rgba(255, 0, 255, 255));
            self.role.map_or(fallback, |role| role.resolve(fallback))
        })
    }

    /// 【主题配置】【角色解析】用 color 和 role 保存启动回退及运行时角色，不覆盖显式颜色，无返回值
    pub(super) fn or(&mut self, color: Rgba, role: crate::theme::Role) {
        // 1. 【主题配置】【旧组件迁移】旧模板内置值继续继承当前主题，真正自定义值保持显式覆盖
        if self
            .explicit
            .is_some_and(|value| super::component_theme::is_builtin(role, value))
        {
            self.explicit = None;
        }
        self.fallback = Some(color);
        self.role = Some(role);
    }
}

impl<'de> Deserialize<'de> for ThemeColor {
    /// 【主题配置】【显式解析】从 d 读取组件颜色，空文字使用继承，返回颜色配置或格式错误
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        if s.trim().is_empty() {
            return Ok(Self::default());
        }
        parse_rgba(&s)
            .map(|c| Self {
                explicit: Some(c),
                ..Self::default()
            })
            .map_err(serde::de::Error::custom)
    }
}

impl Serialize for ThemeColor {
    /// 【主题配置】【配置序列化】向 s 写入显式颜色或空继承值，返回序列化结果，不把启动回退写成覆盖值
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.explicit {
            Some(color) => color.serialize(s),
            None => s.serialize_str(""),
        }
    }
}

pub(super) fn parse_rgba(s: &str) -> Result<Rgba, String> {
    Rgba::parse_hex(s).ok_or_else(|| format!("invalid color {s:?}, expected #RRGGBB or #RRGGBBAA"))
}

impl<'de> Deserialize<'de> for Rgba {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        parse_rgba(&String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

impl Serialize for Rgba {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let Rgba(r, g, b, a) = *self;
        let hex = if a == 255 {
            format!("#{r:02X}{g:02X}{b:02X}")
        } else {
            format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
        };
        s.serialize_str(&hex)
    }
}

impl<'de> Deserialize<'de> for Outputs {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Outputs::parse(&String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

impl Serialize for Outputs {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let letters: String = [
            (self.copy, 'c'),
            (self.pin, 'p'),
            (self.save, 's'),
            (self.stdout, 'o'),
            (self.primary, 'r'),
        ]
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, letter)| letter)
        .collect();
        s.serialize_str(&letters)
    }
}

// ==========================================
// Sections
// ==========================================
