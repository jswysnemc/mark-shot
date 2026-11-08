//! 【放大镜】【持久数据】保存双框几何与取样比例，不持有或复制原图

use serde::{Deserialize, Serialize};
use tiny_skia::Rect;

pub type Point = (f32, f32);

pub const DEFAULT_ZOOM: f32 = 2.75;
pub const MIN_ZOOM: f32 = 1.25;
pub const MAX_ZOOM: f32 = 6.0;
pub const MIN_DIAMETER: f32 = 72.0;
pub const MAX_DIAMETER: f32 = 320.0;
pub const MIN_DRAG_DISTANCE: f32 = 64.0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MagnifierShape {
    #[default]
    Circle,
    Rectangle,
}

impl MagnifierShape {
    pub const ALL: [Self; 2] = [Self::Circle, Self::Rectangle];

    /// 【放大镜】【形状名称】返回当前形状在配置与状态文件中的稳定名称
    pub fn key(self) -> &'static str {
        match self {
            Self::Circle => "circle",
            Self::Rectangle => "rectangle",
        }
    }

    /// 【放大镜】【形状解析】将 value 转为圆形或矩形；未知名称返回空值
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "circle" => Some(Self::Circle),
            "rectangle" | "rect" => Some(Self::Rectangle),
            _ => None,
        }
    }

    /// 【放大镜】【形状切换】返回另一种框形，不修改现有标注
    pub fn next(self) -> Self {
        match self {
            Self::Circle => Self::Rectangle,
            Self::Rectangle => Self::Circle,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Magnifier {
    pub radius: f32,
    pub source: Point,
    pub center: Point,
    pub size: Point,
    pub shape: MagnifierShape,
    pub zoom: f32,
    /// 【放大镜】【旋转数据】旋转使用弧度，轴心在独立移动或缩放时保持不变
    pub rotation: f32,
    pub pivot: Point,
    pub image_bounds: Rect,
    /// 【放大镜】【像素比例】当前坐标单位对应的原图像素数，用于创建尺寸门槛
    pub pixel_scale: f32,
    /// 【放大镜】【绘制比例】导出坐标相对初始逻辑坐标的比例，不改变倍率
    pub style_scale: f32,
}

impl Magnifier {
    /// 【放大镜】【草稿构造】以 shape、source、zoom、image_bounds 和 pixel_scale 创建空草稿；无效范围、起点或比例返回空值
    pub fn new(
        shape: MagnifierShape,
        source: Point,
        zoom: f32,
        image_bounds: Rect,
        pixel_scale: f32,
    ) -> Option<Self> {
        if !super::geometry::finite(source)
            || !pixel_scale.is_finite()
            || pixel_scale <= 0.0
            || image_bounds.is_empty()
            || source.0 < image_bounds.left()
            || source.0 > image_bounds.right()
            || source.1 < image_bounds.top()
            || source.1 > image_bounds.bottom()
        {
            return None;
        }
        Some(Self {
            radius: 0.0,
            source,
            center: source,
            size: (0.0, 0.0),
            shape,
            zoom: clamped_zoom(zoom),
            rotation: 0.0,
            pivot: source,
            image_bounds,
            pixel_scale,
            style_scale: 1.0,
        })
    }

    /// 【放大镜】【倍率修改】返回使用 zoom 的新标注，镜框不动，源框尺寸与中心保持在原图内
    pub fn with_zoom(self, zoom: f32) -> Self {
        let mut result = Self {
            zoom: clamped_zoom(zoom),
            ..self
        };
        result.source = super::geometry::clamped_center(
            result.source,
            (result.size.0 / result.zoom, result.size.1 / result.zoom),
            result.image_bounds,
        );
        result
    }

    /// 【放大镜】【框形修改】返回 shape 对应的新标注；圆形使用短边并保留双框中心，必要时约束到原图内
    pub fn with_shape(self, shape: MagnifierShape) -> Self {
        let mut result = Self { shape, ..self };
        if shape == MagnifierShape::Circle {
            let side = result.size.0.min(result.size.1);
            result.size = (side, side);
        }
        result.constrained()
    }

    /// 【放大镜】【坐标缩放】按 factor 同比映射全部几何；返回新标注，原图取样尺寸和倍率不变
    pub fn scaled(self, factor: f32) -> Self {
        if !factor.is_finite() || factor <= 0.0 {
            return self;
        }
        let point = |value: Point| (value.0 * factor, value.1 * factor);
        let Some(bounds) = Rect::from_ltrb(
            self.image_bounds.left() * factor,
            self.image_bounds.top() * factor,
            self.image_bounds.right() * factor,
            self.image_bounds.bottom() * factor,
        ) else {
            return self;
        };
        Self {
            source: point(self.source),
            center: point(self.center),
            size: point(self.size),
            pivot: point(self.pivot),
            image_bounds: bounds,
            pixel_scale: self.pixel_scale / factor,
            style_scale: self.style_scale * factor,
            ..self
        }
    }

    /// 【放大镜】【整体移动】按 delta 移动双框与旋转轴心；返回新标注，不移动冻结原图的范围
    pub fn translated(self, delta: Point) -> Self {
        if !super::geometry::finite(delta) {
            return self;
        }
        let point = |value: Point| (value.0 + delta.0, value.1 + delta.1);
        Self {
            source: point(self.source),
            center: point(self.center),
            pivot: point(self.pivot),
            ..self
        }
    }

    /// 【放大镜】【边界映射】以 map 映射双框中心、镜框对角点与轴心；返回新几何，原图范围不变
    pub fn mapped(self, map: impl Fn(Point) -> Point) -> Self {
        let mut result = self.mapped_storage(map);
        if result.shape == MagnifierShape::Circle {
            let side = result.size.0.min(result.size.1);
            result.size = (side, side);
        }
        result
    }

    /// 【放大镜】【存储映射】按map映射完整镜框矩形、源点与轴心，返回保留长宽的新成员，群组缩放允许圆形变为椭圆
    pub(crate) fn mapped_storage(self, map: impl Fn(Point) -> Point) -> Self {
        let start = map((
            self.center.0 - self.size.0 / 2.0,
            self.center.1 - self.size.1 / 2.0,
        ));
        let end = map((
            self.center.0 + self.size.0 / 2.0,
            self.center.1 + self.size.1 / 2.0,
        ));
        let size = ((end.0 - start.0).abs(), (end.1 - start.1).abs());
        Self {
            source: map(self.source),
            center: ((start.0 + end.0) / 2.0, (start.1 + end.1) / 2.0),
            pivot: map(self.pivot),
            size,
            ..self
        }
    }

    /// 【放大镜】【描边尺寸】根据 width 返回向内绘制的边框宽度；逻辑宽度限制为 1.5 至 18，导出同比缩放
    pub fn border_width(self, width: f32) -> f32 {
        let scale = self.style_scale.max(f32::EPSILON);
        if !width.is_finite() {
            return 1.5 * scale;
        }
        (width / scale).clamp(1.5, 18.0) * scale
    }
}

/// 【放大镜】【倍率约束】将 value 限制到原版 1.25 至 6 倍；非有限值返回默认 2.75 倍
pub fn clamped_zoom(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_ZOOM, MAX_ZOOM)
    } else {
        DEFAULT_ZOOM
    }
}
