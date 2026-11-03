//! 【序号】【数据模型】保存标签样式、圆心、尖端和独立计数状态

use serde::{Deserialize, Serialize};

pub const WIDTH: (f32, f32) = (1.0, 72.0);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumberStyle {
    #[default]
    Arabic,
    UpperAlpha,
    LowerAlpha,
    UpperRoman,
    LowerRoman,
    HeavenlyStem,
    Chinese,
}

impl NumberStyle {
    pub const ALL: [Self; 7] = [
        Self::Arabic,
        Self::UpperAlpha,
        Self::LowerAlpha,
        Self::UpperRoman,
        Self::LowerRoman,
        Self::HeavenlyStem,
        Self::Chinese,
    ];

    /// 【序号】【样式名称】返回当前样式的稳定配置名称，不使用本地化界面文字
    pub fn key(self) -> &'static str {
        match self {
            Self::Arabic => "arabic",
            Self::UpperAlpha => "upper_alpha",
            Self::LowerAlpha => "lower_alpha",
            Self::UpperRoman => "upper_roman",
            Self::LowerRoman => "lower_roman",
            Self::HeavenlyStem => "heavenly_stem",
            Self::Chinese => "chinese",
        }
    }

    /// 【序号】【样式解析】将 value 转为稳定样式，接受大小写及连字符；未知名称返回空值
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim().to_ascii_lowercase().replace('-', "_");
        Self::ALL.into_iter().find(|style| style.key() == value)
    }

    /// 【序号】【样式说明】返回当前样式在统一选择面板中的说明文字
    pub fn description(self) -> &'static str {
        match self {
            Self::Arabic => "Arabic numerals",
            Self::UpperAlpha => "Uppercase letters",
            Self::LowerAlpha => "Lowercase letters",
            Self::UpperRoman => "Uppercase Roman numerals",
            Self::LowerRoman => "Lowercase Roman numerals",
            Self::HeavenlyStem => "Heavenly stems",
            Self::Chinese => "Chinese numerals",
        }
    }

    /// 【序号】【样式预览】返回当前样式的短标签，用于属性按钮和网格预览
    pub fn preview(self) -> &'static str {
        match self {
            Self::Arabic => "123",
            Self::UpperAlpha => "ABC",
            Self::LowerAlpha => "abc",
            Self::UpperRoman => "IV",
            Self::LowerRoman => "iv",
            Self::HeavenlyStem => "甲乙丙",
            Self::Chinese => "一二三",
        }
    }

    /// 【序号】【中文样式】返回当前样式是否使用随包提供的中文序号字形
    pub fn is_cjk(self) -> bool {
        matches!(self, Self::HeavenlyStem | Self::Chinese)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberMark {
    pub tip: (f32, f32),
    pub bubble: (f32, f32),
    pub number: u32,
    pub style: NumberStyle,
    /// 【序号】【旋转轴心】以圆心为轴保存弧度，尖端坐标保留在未旋转空间
    pub rotation: f32,
    /// 【序号】【导出比例】缩放固定圆框留量和边框限制，不改变编号语义
    pub style_scale: f32,
}

impl NumberMark {
    /// 【序号】【标记构造】使用 bubble、number、style 创建单击圆框；非有限圆心或零编号返回空值
    pub fn new(bubble: (f32, f32), number: u32, style: NumberStyle) -> Option<Self> {
        if !bubble.0.is_finite() || !bubble.1.is_finite() || number == 0 {
            return None;
        }
        Some(Self {
            tip: bubble,
            bubble,
            number,
            style,
            rotation: 0.0,
            style_scale: 1.0,
        })
    }

    /// 【序号】【数据校验】返回几何、编号和导出比例是否有效，不加载字体或分配像素
    pub fn valid(self) -> bool {
        [
            self.tip.0,
            self.tip.1,
            self.bubble.0,
            self.bubble.1,
            self.rotation,
            self.style_scale,
        ]
        .into_iter()
        .all(f32::is_finite)
            && self.style_scale > 0.0
            && self.number > 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NumberSequence {
    next: Option<u32>,
}

impl Default for NumberSequence {
    /// 【序号】【计数初始化】返回从一开始的会话计数，不读取其他工具或磁盘偏好
    fn default() -> Self {
        Self { next: Some(1) }
    }
}

impl NumberSequence {
    /// 【序号】【下一编号】返回尚未使用的编号，计数耗尽时返回空值
    pub fn next(self) -> Option<u32> {
        self.next
    }

    /// 【序号】【提交计数】按已提交 number 推进计数，不回退到旧编号；达到整数上限后停止，无返回值
    pub fn advance(&mut self, number: u32) {
        if let Some(current) = self.next {
            self.next = number.checked_add(1).map(|next| next.max(current));
        }
    }

    /// 【序号】【重置计数】把下一编号恢复为一，保留已有标注；返回计数是否变化
    pub fn reset(&mut self) -> bool {
        let changed = self.next != Some(1);
        self.next = Some(1);
        changed
    }
}

/// 【序号】【笔宽约束】把 width 限定为 style_scale 对应的一到七十二像素；非有限值使用三像素默认值，返回有效笔宽
pub fn clamped_width(width: f32, style_scale: f32) -> f32 {
    let unit = if style_scale.is_finite() && style_scale > 0.0 {
        style_scale
    } else {
        1.0
    };
    if width.is_finite() {
        width.clamp(WIDTH.0 * unit, WIDTH.1 * unit)
    } else {
        3.0 * unit
    }
}
