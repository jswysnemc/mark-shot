//! 【箭头】【组合属性】头部、尾部和双向分别表达，旧样式常量仅保留读取兼容

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArrowHead {
    Fletched,
    Kde,
    #[default]
    Direct,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArrowTail {
    Pointed,
    #[default]
    Rounded,
}

impl ArrowTail {
    /// 【箭头】【尾部标识】无参数，返回持久化使用的稳定名称
    pub fn key(self) -> &'static str {
        match self {
            Self::Pointed => "pointed",
            Self::Rounded => "rounded",
        }
    }

    /// 【箭头】【尾部读取】按 value 读取尾部，未知值返回空值
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pointed" | "sharp" => Some(Self::Pointed),
            "rounded" | "round" => Some(Self::Rounded),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ArrowStyle {
    pub head: ArrowHead,
    pub tail: ArrowTail,
    pub bidirectional: bool,
}

#[allow(non_upper_case_globals)]
impl ArrowStyle {
    pub const Fletched: Self = Self::new(ArrowHead::Fletched, false);
    pub const Kde: Self = Self::new(ArrowHead::Kde, false);
    pub const Direct: Self = Self::new(ArrowHead::Direct, false);
    pub const BidirectionalFletched: Self = Self::new(ArrowHead::Fletched, true);
    pub const BidirectionalKde: Self = Self::new(ArrowHead::Kde, true);
    pub const BidirectionalDirect: Self = Self::new(ArrowHead::Direct, true);
    pub const ALL: [Self; 3] = [Self::Fletched, Self::Kde, Self::Direct];

    /// 【箭头】【属性构造】按 head 和 bidirectional 返回圆形尾部的组合属性
    const fn new(head: ArrowHead, bidirectional: bool) -> Self {
        Self {
            head,
            tail: ArrowTail::Rounded,
            bidirectional,
        }
    }

    /// 【箭头】【头部标识】返回当前头部的稳定配置名称
    pub fn key(self) -> &'static str {
        match self.head {
            ArrowHead::Fletched => "fletched",
            ArrowHead::Kde => "kde",
            ArrowHead::Direct => "direct",
        }
    }

    /// 【箭头】【头部说明】返回当前头部在紧凑选择面板中的标签
    pub fn label(self) -> &'static str {
        match self.head {
            ArrowHead::Fletched => "Fletched",
            ArrowHead::Kde => "KDE",
            ArrowHead::Direct => "Direct",
        }
    }

    /// 【箭头】【头部索引】无参数，返回三种头部选项中的位置，方向和尾部不影响索引
    pub fn head_index(self) -> usize {
        match self.head {
            ArrowHead::Fletched => 0,
            ArrowHead::Kde => 1,
            ArrowHead::Direct => 2,
        }
    }

    /// 【箭头】【头部替换】按 style 替换头部，返回保留当前方向和尾部的属性
    pub fn with_head(self, style: Self) -> Self {
        Self {
            head: style.head,
            ..self
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
        self.bidirectional
    }

    /// 【箭头】【开口判断】返回当前头部是否为 KDE 开口形状
    pub fn open(self) -> bool {
        self.head == ArrowHead::Kde
    }

    /// 【箭头】【直箭头判断】返回当前头部是否使用内凹的实心直箭头
    pub fn direct(self) -> bool {
        self.head == ArrowHead::Direct
    }
}
