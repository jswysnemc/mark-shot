//! 【区域识别】【候选模型】图像像素矩形、边界质量与检测来源

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PixelRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl PixelRect {
    /// 【区域识别】【矩形构造】按 x、y、width、height 创建右下边界不包含在内的像素矩形，返回矩形
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            left: x,
            top: y,
            right: x.saturating_add(width),
            bottom: y.saturating_add(height),
        }
    }

    /// 【区域识别】【矩形宽度】无参数，返回左右边界之间的像素数
    pub fn width(self) -> i32 {
        self.right - self.left
    }

    /// 【区域识别】【矩形高度】无参数，返回上下边界之间的像素数
    pub fn height(self) -> i32 {
        self.bottom - self.top
    }

    /// 【区域识别】【矩形面积】无参数，返回整数像素面积，避免普通整数乘法溢出
    pub fn area(self) -> i64 {
        i64::from(self.width()) * i64::from(self.height())
    }

    /// 【区域识别】【像素命中】判断 point 是否属于矩形，返回命中结果，右下边界保持排除
    pub fn contains(self, point: (i32, i32)) -> bool {
        point.0 >= self.left && point.0 < self.right && point.1 >= self.top && point.1 < self.bottom
    }

    /// 【区域识别】【完整包含】判断 child 是否完整位于本矩形内，返回包含结果
    pub fn contains_rect(self, child: Self) -> bool {
        self.left <= child.left
            && self.top <= child.top
            && self.right >= child.right
            && self.bottom >= child.bottom
    }

    /// 【区域识别】【边界扩展】按 amount 向四边扩展像素范围，返回扩展矩形
    pub fn expanded(self, amount: i32) -> Self {
        Self {
            left: self.left.saturating_sub(amount),
            top: self.top.saturating_sub(amount),
            right: self.right.saturating_add(amount),
            bottom: self.bottom.saturating_add(amount),
        }
    }

    /// 【区域识别】【矩形裁切】按 bounds 求交集，返回交集，无交集时返回空值
    pub fn intersect(self, bounds: Self) -> Option<Self> {
        let clipped = Self {
            left: self.left.max(bounds.left),
            top: self.top.max(bounds.top),
            right: self.right.min(bounds.right),
            bottom: self.bottom.min(bounds.bottom),
        };
        (clipped.right > clipped.left && clipped.bottom > clipped.top).then_some(clipped)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Contour,
    Component,
    Background,
    Lines,
    Window,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub rect: PixelRect,
    pub quality: f32,
    pub sides: [f32; 4],
    pub interior_edges: f32,
    pub sources: Vec<Source>,
}

impl Candidate {
    /// 【区域识别】【来源判断】无参数，返回候选是否含合成器窗口来源，合并后的窗口来源仍生效
    pub fn is_window(&self) -> bool {
        self.sources.contains(&Source::Window)
    }

    /// 【区域识别】【候选排序】无参数，返回边界质量和尺度的排名，数值越小越优先
    pub(super) fn rank(&self) -> f32 {
        let tier = if self.quality >= 0.80 { 0.0 } else { 10000.0 };
        tier + (self.rect.area() as f32).sqrt().sqrt() / self.quality.max(0.1).powf(2.0)
    }
}

/// 【区域识别】【候选合并】按 tolerance 比较 candidates 与 candidate，合并边界相近项并保留所有来源，无返回值
pub(super) fn merge(candidates: &mut Vec<Candidate>, candidate: Candidate, tolerance: i32) {
    for existing in candidates.iter_mut() {
        let a = existing.rect;
        let b = candidate.rect;
        let near = [
            (a.left, b.left),
            (a.top, b.top),
            (a.right, b.right),
            (a.bottom, b.bottom),
        ]
        .iter()
        .all(|(a, b)| (i64::from(*a) - i64::from(*b)).abs() <= i64::from(tolerance));
        let ratio = a.area().min(b.area()) as f64 / a.area().max(b.area()) as f64;
        if !near || ratio <= 0.8 || !ratio.is_finite() {
            continue;
        }
        // 1. 【区域识别】【来源保留】质量更高者提供边界和评分，来源顺序保留已有项后再补充新项
        let mut sources = existing.sources.clone();
        for source in &candidate.sources {
            if !sources.contains(source) {
                sources.push(*source);
            }
        }
        if candidate.quality > existing.quality {
            *existing = candidate;
        }
        existing.sources = sources;
        return;
    }
    candidates.push(candidate);
}
