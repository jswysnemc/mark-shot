//! 【尺子】【测量数据】保存逻辑几何、原图像素比例与绘制比例，不持有图像缓冲

use serde::{Deserialize, Serialize};

pub type Point = (f32, f32);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RulerMode {
    #[default]
    Length,
    Angle,
}

impl RulerMode {
    pub const ALL: [Self; 2] = [Self::Length, Self::Angle];

    /// 【尺子】【模式名称】返回配置与状态文件使用的稳定名称
    pub fn key(self) -> &'static str {
        match self {
            Self::Length => "length",
            Self::Angle => "angle",
        }
    }

    /// 【尺子】【模式标签】返回紧凑属性面板使用的文字
    pub fn label(self) -> &'static str {
        match self {
            Self::Length => "Length",
            Self::Angle => "Angle",
        }
    }

    /// 【尺子】【模式切换】返回下一测量模式，不修改已有测量
    pub fn next(self) -> Self {
        match self {
            Self::Length => Self::Angle,
            Self::Angle => Self::Length,
        }
    }

    /// 【尺子】【模式解析】将 value 转为测量模式；未知名称返回空值
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "length" => Some(Self::Length),
            "angle" => Some(Self::Angle),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RulerGeometry {
    Length {
        start: Point,
        end: Point,
        rotation: f32,
    },
    Angle {
        points: [Point; 3],
        count: u8,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measurement {
    pub geometry: RulerGeometry,
    /// 【尺子】【测量比例】每个当前坐标单位对应的原图像素数，导出映射后反向调整
    pub pixel_scale: f32,
    /// 【尺子】【样式比例】原图到当前坐标的比例乘以截图补偿，不参与测量值计算
    pub style_scale: f64,
}

impl Measurement {
    /// 【尺子】【草稿构造】使用 mode、start 和 pixel_scale 创建空测量；非有限坐标或无效比例返回空值
    pub fn new(mode: RulerMode, start: Point, pixel_scale: f32) -> Option<Self> {
        if !start.0.is_finite()
            || !start.1.is_finite()
            || !pixel_scale.is_finite()
            || pixel_scale <= 0.0
        {
            return None;
        }
        Some(Self {
            geometry: match mode {
                RulerMode::Length => RulerGeometry::Length {
                    start,
                    end: start,
                    rotation: 0.0,
                },
                RulerMode::Angle => RulerGeometry::Angle {
                    points: [start; 3],
                    count: 2,
                },
            },
            pixel_scale,
            style_scale: 1.0,
        })
    }

    /// 【尺子】【测量模式】返回当前几何对应的测量模式
    pub fn mode(self) -> RulerMode {
        match self.geometry {
            RulerGeometry::Length { .. } => RulerMode::Length,
            RulerGeometry::Angle { .. } => RulerMode::Angle,
        }
    }

    /// 【尺子】【起点读取】返回长度尺起点或角度尺顶点的当前坐标
    pub fn start(self) -> Point {
        match self.geometry {
            RulerGeometry::Length { start, .. } => start,
            RulerGeometry::Angle { points, .. } => points[0],
        }
    }

    /// 【尺子】【坐标映射】按 map 映射全部定点；返回新的几何，不修改像素与样式比例
    pub fn mapped(self, map: impl Fn(Point) -> Point) -> Self {
        let geometry = match self.geometry {
            RulerGeometry::Length {
                start,
                end,
                rotation,
            } => RulerGeometry::Length {
                start: map(start),
                end: map(end),
                rotation,
            },
            RulerGeometry::Angle { points, count } => RulerGeometry::Angle {
                points: points.map(map),
                count,
            },
        };
        Self { geometry, ..self }
    }

    /// 【尺子】【导出映射】按 factor 同比映射几何与样式；返回新测量，原图像素测量值保持不变
    pub fn scaled(self, factor: f32) -> Self {
        if !factor.is_finite() || factor <= 0.0 {
            return self;
        }
        Self {
            pixel_scale: self.pixel_scale / factor,
            style_scale: self.style_scale * f64::from(factor),
            ..self.mapped(|point| (point.0 * factor, point.1 * factor))
        }
    }

    /// 【尺子】【样式单位】根据 width 返回当前坐标中刻度、数字与标签使用的尺寸单位
    pub fn unit(self, width: f32) -> f32 {
        (f64::from(width_unit(width * self.pixel_scale))
            * self.style_scale.max(f64::from(f32::EPSILON))) as f32
    }
}

/// 【尺子】【粗细比例】根据 width 返回原版尺子的样式放大倍数，范围为 0.75 至 3.5
pub fn width_unit(width: f32) -> f32 {
    (1.0 + (width - 3.0) / 9.0).clamp(0.75, 3.5)
}
