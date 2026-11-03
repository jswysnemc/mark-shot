//! 【形状标记】【形状目录】保留原版稳定编号，并独立维护选择面板显示顺序

use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "kebab-case")]
pub enum MarkerShape {
    #[default]
    Triangle = 0,
    Star,
    Check,
    Cross,
    Diamond,
    Heart,
    Hexagon,
    Circle,
    Square,
    Pentagon,
    Plus,
    ArrowUp,
    Spade,
    Club,
    Lightning,
    Ban,
    Octagon,
    Crescent,
    Pin,
    Flag,
    Bookmark,
    Shield,
    Exclamation,
    SpeechBubble,
}

impl MarkerShape {
    pub const ALL: [Self; 24] = [
        Self::Triangle,
        Self::Star,
        Self::Check,
        Self::Cross,
        Self::Diamond,
        Self::Heart,
        Self::Spade,
        Self::Club,
        Self::Circle,
        Self::Square,
        Self::Pentagon,
        Self::Hexagon,
        Self::Octagon,
        Self::Plus,
        Self::ArrowUp,
        Self::Lightning,
        Self::Ban,
        Self::Crescent,
        Self::Pin,
        Self::Flag,
        Self::Bookmark,
        Self::Shield,
        Self::Exclamation,
        Self::SpeechBubble,
    ];

    /// 【形状标记】【配置名称】返回稳定的小写配置名，不受选择面板顺序影响
    pub fn key(self) -> &'static str {
        match self {
            Self::Triangle => "triangle",
            Self::Star => "star",
            Self::Check => "check",
            Self::Cross => "cross",
            Self::Diamond => "diamond",
            Self::Heart => "heart",
            Self::Hexagon => "hexagon",
            Self::Circle => "circle",
            Self::Square => "square",
            Self::Pentagon => "pentagon",
            Self::Plus => "plus",
            Self::ArrowUp => "arrow-up",
            Self::Spade => "spade",
            Self::Club => "club",
            Self::Lightning => "lightning",
            Self::Ban => "ban",
            Self::Octagon => "octagon",
            Self::Crescent => "crescent",
            Self::Pin => "pin",
            Self::Flag => "flag",
            Self::Bookmark => "bookmark",
            Self::Shield => "shield",
            Self::Exclamation => "exclamation",
            Self::SpeechBubble => "speech-bubble",
        }
    }

    /// 【形状标记】【显示名称】返回原版英文源名称，供属性面板与选择网格显示
    pub fn label(self) -> &'static str {
        match self {
            Self::Triangle => "Triangle",
            Self::Star => "Star",
            Self::Check => "Check",
            Self::Cross => "Cross",
            Self::Diamond => "Diamond",
            Self::Heart => "Heart",
            Self::Hexagon => "Hexagon",
            Self::Circle => "Circle",
            Self::Square => "Square",
            Self::Pentagon => "Pentagon",
            Self::Plus => "Plus",
            Self::ArrowUp => "Arrow Up",
            Self::Spade => "Spade",
            Self::Club => "Club",
            Self::Lightning => "Lightning",
            Self::Ban => "Ban",
            Self::Octagon => "Octagon",
            Self::Crescent => "Crescent",
            Self::Pin => "Pin",
            Self::Flag => "Flag",
            Self::Bookmark => "Bookmark",
            Self::Shield => "Shield",
            Self::Exclamation => "Exclamation",
            Self::SpeechBubble => "Speech Bubble",
        }
    }

    /// 【形状标记】【旧编号转换】将原版整数 value 转为形状；未知或越界编号返回空值
    pub fn from_legacy_id(value: i64) -> Option<Self> {
        Self::ALL.into_iter().find(|shape| *shape as i64 == value)
    }
}

impl FromStr for MarkerShape {
    type Err = String;

    /// 【形状标记】【名称解析】将 value 的稳定配置名转为形状；未知名称返回错误
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim().to_ascii_lowercase().replace('_', "-");
        Self::ALL
            .into_iter()
            .find(|shape| shape.key() == normalized)
            .ok_or_else(|| format!("unknown marker shape: {value}"))
    }
}
