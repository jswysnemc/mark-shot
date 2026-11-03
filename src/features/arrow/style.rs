//! 【箭头】【头部类型】保持原版四种样式编号，头部与箭身线型独立组合

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(usize)]
pub enum ArrowStyle {
    #[default]
    Fletched,
    Kde,
    BidirectionalFletched,
    BidirectionalKde,
}

impl ArrowStyle {
    pub const ALL: [Self; 4] = [
        Self::Fletched,
        Self::Kde,
        Self::BidirectionalFletched,
        Self::BidirectionalKde,
    ];

    /// 【箭头】【头部标识】返回当前头部的稳定配置名称
    pub fn key(self) -> &'static str {
        match self {
            Self::Fletched => "fletched",
            Self::Kde => "kde",
            Self::BidirectionalFletched => "bidirectional_fletched",
            Self::BidirectionalKde => "bidirectional_kde",
        }
    }

    /// 【箭头】【头部说明】返回当前头部在紧凑选择面板中的标签
    pub fn label(self) -> &'static str {
        match self {
            Self::Fletched => "Fletched",
            Self::Kde => "KDE",
            Self::BidirectionalFletched => "Double fletched",
            Self::BidirectionalKde => "Double KDE",
        }
    }

    /// 【箭头】【头部读取】把 value 转为头部样式，兼容原版编号；返回样式或空值
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "fletched" | "filled" | "0" => Some(Self::Fletched),
            "kde" | "open" | "1" => Some(Self::Kde),
            "bidirectional_fletched" | "double_fletched" | "2" => Some(Self::BidirectionalFletched),
            "bidirectional_kde" | "double_kde" | "3" => Some(Self::BidirectionalKde),
            _ => None,
        }
    }

    /// 【箭头】【双向判断】返回当前头部是否绘制两个尖端
    pub fn bidirectional(self) -> bool {
        matches!(self, Self::BidirectionalFletched | Self::BidirectionalKde)
    }

    /// 【箭头】【开口判断】返回当前头部是否为 KDE 开口形状
    pub fn open(self) -> bool {
        matches!(self, Self::Kde | Self::BidirectionalKde)
    }
}
