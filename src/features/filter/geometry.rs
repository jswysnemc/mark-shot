//! 【滤镜】【坐标几何】统一原图网格、旋转路径和正文边界

use tiny_skia::{IntRect, Path, PathBuilder, Rect};

use super::RegionFilter;
use super::model::Point;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelGrid {
    pub bounds: Rect,
    pub scale: f32,
    pub width: u32,
    pub height: u32,
}

impl PixelGrid {
    /// 【滤镜】【原图网格】以 bounds、scale、width、height 创建原图像素坐标；无效范围返回空值，不分配像素
    pub fn new(bounds: Rect, scale: f32, width: u32, height: u32) -> Option<Self> {
        if !scale.is_finite()
            || scale <= 0.0
            || width == 0
            || height == 0
            || width > i32::MAX as u32
            || height > i32::MAX as u32
            || bounds.is_empty()
        {
            return None;
        }
        Some(Self {
            bounds,
            scale,
            width,
            height,
        })
    }

    /// 【滤镜】【像素边界】返回原图左闭右开整数范围，不受局部重绘尺寸影响
    pub fn pixels(self) -> IntRect {
        IntRect::from_xywh(0, 0, self.width, self.height).unwrap()
    }

    /// 【滤镜】【区域对齐】把 filter 未旋转矩形向外对齐到原图像素；返回裁入原图后的整数区域
    pub fn region(self, filter: RegionFilter) -> Option<IntRect> {
        let rect = filter.rect()?;
        let (left, top) = self.to_pixel((f64::from(rect.left()), f64::from(rect.top())));
        let (right, bottom) = self.to_pixel((f64::from(rect.right()), f64::from(rect.bottom())));
        let left = pixel_edge(left).floor().clamp(0.0, f64::from(self.width)) as i32;
        let top = pixel_edge(top).floor().clamp(0.0, f64::from(self.height)) as i32;
        let right = pixel_edge(right).ceil().clamp(0.0, f64::from(self.width)) as i32;
        let bottom = pixel_edge(bottom).ceil().clamp(0.0, f64::from(self.height)) as i32;
        IntRect::from_ltrb(left, top, right, bottom)
    }

    /// 【滤镜】【逻辑像素映射】把 point 转为原图连续像素坐标；返回边界坐标，像素中心带半像素偏移
    pub fn to_pixel(self, point: (f64, f64)) -> (f64, f64) {
        (
            (point.0 - f64::from(self.bounds.left())) * f64::from(self.scale),
            (point.1 - f64::from(self.bounds.top())) * f64::from(self.scale),
        )
    }

    /// 【滤镜】【像素逻辑映射】把原图连续 x、y 映射回绘图坐标；返回逻辑位置，不读取像素
    pub fn point(self, x: f64, y: f64) -> Point {
        (
            (f64::from(self.bounds.left()) + x / f64::from(self.scale)) as f32,
            (f64::from(self.bounds.top()) + y / f64::from(self.scale)) as f32,
        )
    }

    /// 【滤镜】【像素矩形映射】把 rect 映射为原图对应的逻辑矩形；返回范围或无效几何
    pub fn rect(self, rect: IntRect) -> Option<Rect> {
        let start = self.point(f64::from(rect.left()), f64::from(rect.top()));
        let end = self.point(f64::from(rect.right()), f64::from(rect.bottom()));
        Rect::from_ltrb(start.0, start.1, end.0, end.1)
    }
}

/// 【滤镜】【浮点边界】消除坐标缩放在整数边界附近的舍入误差；返回稳定边界，不吸附真实亚像素位置
fn pixel_edge(value: f64) -> f64 {
    if (value - value.round()).abs() < 0.0001 {
        value.round()
    } else {
        value
    }
}

/// 【滤镜】【矩形顶点】按 rect 返回顺时针四顶点，用于正文、控制框和旋转边界
pub fn corners(rect: Rect) -> [Point; 4] {
    [
        (rect.left(), rect.top()),
        (rect.right(), rect.top()),
        (rect.right(), rect.bottom()),
        (rect.left(), rect.bottom()),
    ]
}

impl RegionFilter {
    /// 【滤镜】【视觉坐标】把未旋转 point 绕矩形中心旋转；返回当前视觉位置
    pub fn to_visual(self, point: Point) -> Point {
        rotate(point, self.center(), self.rotation)
    }

    /// 【滤镜】【局部坐标】把视觉 point 反旋转到原图采样空间；返回未旋转位置
    pub fn to_local(self, point: Point) -> Point {
        rotate(point, self.center(), -self.rotation)
    }

    /// 【滤镜】【高精度局部坐标】把视觉 point 反旋转；返回双精度位置，避免分块插值改变像素相位
    pub fn local_f64(self, point: (f64, f64)) -> (f64, f64) {
        let center = self.center();
        rotate_f64(
            point,
            (f64::from(center.0), f64::from(center.1)),
            -f64::from(self.rotation),
        )
    }

    /// 【滤镜】【旋转正文边界】返回完整未裁切矩形的旋转包围盒；无面积或无效数据返回空值
    pub fn bounds(self) -> Option<Rect> {
        bounds_of(corners(self.rect()?).map(|point| self.to_visual(point)))
    }

    /// 【滤镜】【矩形路径】按 rect 和当前旋转创建闭合路径；返回正文或控制框路径
    pub fn path(self, rect: Rect) -> Option<Path> {
        let points = corners(rect).map(|point| self.to_visual(point));
        let mut builder = PathBuilder::new();
        builder.move_to(points[0].0, points[0].1);
        for point in &points[1..] {
            builder.line_to(point.0, point.1);
        }
        builder.close();
        builder.finish()
    }
}

/// 【滤镜】【顶点边界】由 points 计算轴对齐包围盒；返回有效矩形或空值
pub fn bounds_of(points: [Point; 4]) -> Option<Rect> {
    Rect::from_ltrb(
        points
            .iter()
            .map(|point| point.0)
            .fold(f32::INFINITY, f32::min),
        points
            .iter()
            .map(|point| point.1)
            .fold(f32::INFINITY, f32::min),
        points
            .iter()
            .map(|point| point.0)
            .fold(f32::NEG_INFINITY, f32::max),
        points
            .iter()
            .map(|point| point.1)
            .fold(f32::NEG_INFINITY, f32::max),
    )
}

/// 【滤镜】【旋转变换】将 point 绕 center 旋转 radians；返回逻辑坐标，不改变源矩形
fn rotate(point: Point, center: Point, radians: f32) -> Point {
    let result = rotate_f64(
        (f64::from(point.0), f64::from(point.1)),
        (f64::from(center.0), f64::from(center.1)),
        f64::from(radians),
    );
    (result.0 as f32, result.1 as f32)
}

/// 【滤镜】【双精度旋转】将 point 绕 center 旋转 radians；返回高精度位置，供原图插值使用
fn rotate_f64(point: (f64, f64), center: (f64, f64), radians: f64) -> (f64, f64) {
    let (sine, cosine) = radians.sin_cos();
    let delta = (point.0 - center.0, point.1 - center.1);
    (
        center.0 + delta.0 * cosine - delta.1 * sine,
        center.1 + delta.0 * sine + delta.1 * cosine,
    )
}
