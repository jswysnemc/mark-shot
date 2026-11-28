//! 【尺子】【读数换行】按字段生成有限的换行组合，字号和原图测量值保持不变

use super::{Measurement, Readout, Readouts};

pub(crate) const PAD_X: f64 = 10.0;
pub(crate) const PAD_Y: f64 = 5.0;
pub(crate) const ROW_GAP: f64 = 2.0;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Flow {
    pub lines: Vec<String>,
    pub size: (f64, f64),
}

/// 【尺子】【排版尺寸】按 lines、窗口字号 pixels 和样式单位 unit 返回最长行宽与总行高，包含内边距
pub(super) fn size(lines: &[String], pixels: f32, unit: f64) -> (f64, f64) {
    let mut width = 0.0_f32;
    let mut height = 0.0_f32;
    for line in lines {
        let metrics = crate::fonts::ruler::measure(line, pixels);
        width = width.max(metrics.width.ceil());
        height = height.max(metrics.height.ceil());
    }
    let rows = lines.len() as f64;
    (
        f64::from(width) + 2.0 * PAD_X * unit,
        f64::from(height) * rows + (2.0 * PAD_Y + (rows - 1.0) * ROW_GAP) * unit,
    )
}

/// 【尺子】【分行组合】按 measurement 勾选的字段生成多行候选，pixels 和 unit 用于实际度量，不拆分数值或单位
pub(super) fn wraps(measurement: Measurement, pixels: f32, unit: f64) -> Vec<Flow> {
    let Some(metrics) = measurement.length_metrics() else {
        return Vec::new();
    };
    let fields: Vec<_> = [
        Readout::Width,
        Readout::Height,
        Readout::Diagonal,
        Readout::Area,
    ]
    .into_iter()
    .filter(|&field| measurement.readouts.contains(field))
    .collect();
    let mut flows = Vec::new();
    // 1. 【尺子】【断行位置】最多四个字段、三个可选断点，同行的宽高沿用紧凑的尺寸写法
    for breaks in 1..(1 << fields.len().saturating_sub(1)) {
        let mut lines = Vec::new();
        let mut row = Readouts::default();
        for (index, &field) in fields.iter().enumerate() {
            *row.field_mut(field) = true;
            if index + 1 == fields.len() || breaks & (1 << index) != 0 {
                lines.push(row.text(metrics).expect("nonempty ruler row"));
                row = Readouts::default();
            }
        }
        flows.push(Flow {
            size: size(&lines, pixels, unit),
            lines,
        });
    }
    flows
}
