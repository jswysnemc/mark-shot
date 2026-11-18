//! 【荧光笔】【绘制模式】默认直线模式与旧版 Qt 保持一致

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HighlighterStyle {
    Freehand,
    #[default]
    StraightLine,
}

impl HighlighterStyle {
    /// 【荧光笔】【模式标识】无参数，返回稳定持久化名称
    pub fn key(self) -> &'static str {
        match self {
            Self::Freehand => "freehand",
            Self::StraightLine => "straight_line",
        }
    }

    /// 【荧光笔】【模式读取】按 value 返回模式，未知值返回空值
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "freehand" | "pen" => Some(Self::Freehand),
            "straight_line" | "line" => Some(Self::StraightLine),
            _ => None,
        }
    }
}
