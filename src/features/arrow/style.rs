//! 【箭头】【头部类型】保留原版四种样式编号，并新增 PixPin 风格的直箭头头部

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(usize)]
pub enum ArrowStyle {
    Fletched,
    Kde,
    BidirectionalFletched,
    BidirectionalKde,
    #[default]
    Direct,
    BidirectionalDirect,
}

impl ArrowStyle {
    pub const ALL: [Self; 6] = [
        Self::Fletched,
        Self::Kde,
        Self::BidirectionalFletched,
        Self::BidirectionalKde,
        Self::Direct,
        Self::BidirectionalDirect,
    ];

    /// 【箭头】【头部标识】返回当前头部的稳定配置名称
    pub fn key(self) -> &'static str {
        match self {
            Self::Fletched => "fletched",
            Self::Kde => "kde",
            Self::BidirectionalFletched => "bidirectional_fletched",
            Self::BidirectionalKde => "bidirectional_kde",
            Self::Direct => "direct",
            Self::BidirectionalDirect => "bidirectional_direct",
        }
    }

    /// 【箭头】【头部说明】返回当前头部在紧凑选择面板中的标签
    pub fn label(self) -> &'static str {
        match self {
            Self::Fletched => "Fletched",
            Self::Kde => "KDE",
            Self::BidirectionalFletched => "Double fletched",
            Self::BidirectionalKde => "Double KDE",
            Self::Direct => "Direct",
            Self::BidirectionalDirect => "Double direct",
        }
    }

    /// 【箭头】【头部读取】把 value 转为头部样式，兼容原版编号；返回样式或空值
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "fletched" | "filled" | "0" => Some(Self::Fletched),
            "kde" | "open" | "1" => Some(Self::Kde),
            "bidirectional_fletched" | "double_fletched" | "2" => Some(Self::BidirectionalFletched),
            "bidirectional_kde" | "double_kde" | "3" => Some(Self::BidirectionalKde),
            "direct" | "dart" | "4" => Some(Self::Direct),
            "bidirectional_direct" | "double_direct" | "5" => Some(Self::BidirectionalDirect),
            _ => None,
        }
    }

    /// 【箭头】【双向判断】返回当前头部是否绘制两个尖端
    pub fn bidirectional(self) -> bool {
        matches!(
            self,
            Self::BidirectionalFletched | Self::BidirectionalKde | Self::BidirectionalDirect
        )
    }

    /// 【箭头】【开口判断】返回当前头部是否为 KDE 开口形状
    pub fn open(self) -> bool {
        matches!(self, Self::Kde | Self::BidirectionalKde)
    }

    /// 【箭头】【直箭头判断】返回当前头部是否使用内凹的实心直箭头
    pub fn direct(self) -> bool {
        matches!(self, Self::Direct | Self::BidirectionalDirect)
    }
}
