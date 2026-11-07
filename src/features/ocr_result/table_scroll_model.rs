//! 【文字识别】【表格逐项滚动】列和行使用离散索引，末端对齐视区，保留小刻度累计

use super::{table_geometry::ROW_HEIGHT, table_grid_layout::Grid};
use crate::{backend::window::ScrollInput, keys::Mods};

#[derive(Clone)]
pub(super) struct Range {
    pub page: usize,
    pub maximum: usize,
    offsets: Offsets,
}

#[derive(Clone)]
enum Offsets {
    Columns(Vec<f32>),
    Rows { limit: f32 },
}

#[derive(Default)]
pub(super) struct Wheel {
    residual: [f64; 2],
}

impl Range {
    /// 【文字识别】【滚动范围】用 widths 和 viewport 创建原版逐项范围，尾部完整可见项决定 page，返回边界及像素位置
    pub(super) fn new(widths: impl Iterator<Item = f32>, viewport: f32) -> Self {
        let widths: Vec<_> = widths.collect();
        let mut visible = 0.0;
        let mut page = 0;
        for width in widths.iter().rev() {
            visible += width;
            if visible > viewport {
                break;
            }
            page += 1;
        }
        let page = page.max(1).min(widths.len().max(1));
        let maximum = widths.len().saturating_sub(page);
        let limit = (widths.iter().sum::<f32>() - viewport).max(0.0);
        let mut offsets = vec![0.0];
        for width in widths.iter().take(maximum) {
            offsets.push((offsets.last().copied().unwrap() + width).min(limit));
        }
        Self {
            page,
            maximum,
            offsets: Offsets::Columns(offsets),
        }
    }

    /// 【文字识别】【横向范围】按 grid 返回逐列滚动范围，超宽列不改变末尾完整可见列计数
    pub fn horizontal(grid: &Grid) -> Self {
        Self::new(
            (0..grid.columns()).map(|column| grid.column(column).1),
            grid.viewport.width(),
        )
    }

    /// 【文字识别】【纵向范围】按 grid、rows 返回逐行滚动范围，最后一行底部与视区对齐
    pub fn vertical(grid: &Grid, rows: usize) -> Self {
        let page = ((grid.viewport.height() / ROW_HEIGHT).floor() as usize)
            .max(1)
            .min(rows.max(1));
        Self {
            page,
            maximum: rows.saturating_sub(page),
            offsets: Offsets::Rows {
                limit: (rows as f32 * ROW_HEIGHT - grid.viewport.height()).max(0.0),
            },
        }
    }

    /// 【文字识别】【索引位置】将 value 限制在最大索引后返回实际像素偏移，末项不会越过内容边缘
    pub fn offset(&self, value: usize) -> f32 {
        let index = value.min(self.maximum);
        match &self.offsets {
            Offsets::Columns(offsets) => offsets[index],
            Offsets::Rows { limit } => (index as f32 * ROW_HEIGHT).min(*limit),
        }
    }

    /// 【文字识别】【位置索引】将 offset 映射到最近的真实索引，返回合法值，供窗口缩放和程序导航恢复
    pub fn index(&self, offset: f32) -> usize {
        match &self.offsets {
            Offsets::Columns(offsets) => offsets
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| (*a - offset).abs().total_cmp(&(*b - offset).abs()))
                .map_or(0, |(index, _)| index),
            Offsets::Rows { .. } => {
                let candidate = ((offset.max(0.0) / ROW_HEIGHT).round() as usize).min(self.maximum);
                if (self.offset(self.maximum) - offset).abs()
                    < (self.offset(candidate) - offset).abs()
                {
                    self.maximum
                } else {
                    candidate
                }
            }
        }
    }

    /// 【文字识别】【刻度移动】按 value、steps 计算新索引，负值向前，超界固定在边缘
    pub fn moved(&self, value: usize, steps: i32) -> usize {
        value
            .saturating_add_signed(steps as isize)
            .min(self.maximum)
    }
}

impl Wheel {
    /// 【文字识别】【刻度累积】按 input、两方向 ranges 返回本次实际项数，Shift 或 Ctrl 每刻度移动一页，普通刻度最多移动一页
    pub fn steps(&mut self, input: ScrollInput, ranges: [&Range; 2]) -> [i32; 2] {
        let angles = [input.angle.0, input.angle.1];
        let mut steps = [0; 2];
        for axis in 0..2 {
            let angle = -angles[axis];
            if !angle.is_finite() || angle == 0.0 {
                continue;
            }
            if input.mods.shift || input.mods.ctrl {
                self.residual[axis] = 0.0;
                steps[axis] =
                    ((angle / 120.0).clamp(-1.0, 1.0) * ranges[axis].page as f64).trunc() as i32;
            } else {
                if self.residual[axis].is_sign_positive() != angle.is_sign_positive() {
                    self.residual[axis] = 0.0;
                }
                self.residual[axis] += angle / 120.0 * 3.0;
                let whole = self.residual[axis].trunc();
                self.residual[axis] -= whole;
                steps[axis] =
                    whole.clamp(-(ranges[axis].page as f64), ranges[axis].page as f64) as i32;
            }
        }
        steps
    }

    /// 【文字识别】【边缘累计清理】按 axis 清除无法滚动方向的小刻度，避免到边缘后反向滚动延迟，无返回值
    pub fn reset_axis(&mut self, axis: usize) {
        self.residual[axis] = 0.0;
    }
}

/// 【文字识别】【旧滚轮参数】按 delta 和 mods 创建兼容旧事件的输入，不伪造设备来源
pub(super) fn legacy(delta: (f64, f64), mods: Mods) -> ScrollInput {
    ScrollInput {
        mods,
        ..ScrollInput::legacy((0.0, 0.0), delta)
    }
}

#[cfg(test)]
pub(super) mod tests;
