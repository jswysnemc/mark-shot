//! 【尺子】【测量信息】管理标准刻度尺可选的原图像素读数

use serde::{Deserialize, Serialize};

pub type LengthMetrics = (u64, u64, u64, u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Readout {
    Width,
    Height,
    Diagonal,
    Area,
}

impl Readout {
    /// 【尺子】【字段名称】返回属性工具条使用的文案键
    pub const fn label(self) -> &'static str {
        match self {
            Self::Width => "Width",
            Self::Height => "Height",
            Self::Diagonal => "Diagonal",
            Self::Area => "Area",
        }
    }

    /// 【尺子】【字段读数】从 metrics 取出对应的原图像素数值及其单位
    pub fn value(self, metrics: LengthMetrics) -> (u64, &'static str) {
        match self {
            Self::Width => (metrics.0, "px"),
            Self::Height => (metrics.1, "px"),
            Self::Diagonal => (metrics.2, "px"),
            Self::Area => (metrics.3, "px²"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Readouts {
    pub width: bool,
    pub height: bool,
    pub diagonal: bool,
    pub area: bool,
}

impl Readouts {
    pub const ALL: Self = Self {
        width: true,
        height: true,
        diagonal: true,
        area: true,
    };

    /// 【尺子】【字段状态】按 field 返回对应开关的可变引用，不影响其他字段
    pub fn field_mut(&mut self, field: Readout) -> &mut bool {
        match field {
            Readout::Width => &mut self.width,
            Readout::Height => &mut self.height,
            Readout::Diagonal => &mut self.diagonal,
            Readout::Area => &mut self.area,
        }
    }

    /// 【尺子】【字段查询】按 field 返回该项是否显示在画布上
    pub fn contains(mut self, field: Readout) -> bool {
        *self.field_mut(field)
    }

    /// 【尺子】【详情文字】将 metrics 中启用的字段组合为一行，未启用任何字段时返回空值
    pub(super) fn text(self, (width, height, diagonal, area): LengthMetrics) -> Option<String> {
        let mut parts = Vec::new();
        match (self.width, self.height) {
            (true, true) => parts.push(format!("{width} x {height} px")),
            (true, false) => parts.push(format!("width {width} px")),
            (false, true) => parts.push(format!("height {height} px")),
            (false, false) => {}
        }
        if self.diagonal {
            parts.push(format!("diag {diagonal} px"));
        }
        if self.area {
            parts.push(format!("area {area} px"));
        }
        (!parts.is_empty()).then(|| parts.join("   "))
    }
}
