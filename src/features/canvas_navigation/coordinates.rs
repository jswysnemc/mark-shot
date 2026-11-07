//! 【画布导航】【坐标映射】保留双精度原图与窗口坐标，不修改标注几何

pub type Point = (f64, f64);
pub type Size = (u32, u32);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    /// 【画布导航】【范围判断】按point返回是否位于当前矩形，包含原版浮点矩形的右边界和下边界
    pub fn contains(self, point: Point) -> bool {
        self.width > 0.0
            && self.height > 0.0
            && point.0 >= self.x
            && point.1 >= self.y
            && point.0 <= self.x + self.width
            && point.1 <= self.y + self.height
    }

    /// 【画布导航】【绘制适配】返回tiny-skia矩形，浮点溢出或空范围返回空值，无参数
    pub fn raster(self) -> Option<tiny_skia::Rect> {
        tiny_skia::Rect::from_xywh(
            self.x as f32,
            self.y as f32,
            self.width as f32,
            self.height as f32,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection {
    pub image_size: Size,
    pub widget_rect: Rect,
}

impl Projection {
    /// 【画布导航】【原图约束】按point返回最后一个原图像素以内的坐标，边界以image_size为准
    pub fn clamp(self, point: Point) -> Point {
        (
            point
                .0
                .clamp(0.0, f64::from(self.image_size.0.saturating_sub(1))),
            point
                .1
                .clamp(0.0, f64::from(self.image_size.1.saturating_sub(1))),
        )
    }

    /// 【画布导航】【窗口转原图】按point返回原图坐标，沿用原版最后一个像素约束，无效范围返回空值
    pub fn widget_to_image(self, point: Point) -> Option<Point> {
        self.widget_to_unclamped_image(point)
            .map(|point| self.clamp(point))
    }

    /// 【画布导航】【未裁切映射】按point返回未裁切原图坐标，供边缘拖动求值，空范围或非有限点返回空值
    pub fn widget_to_unclamped_image(self, point: Point) -> Option<Point> {
        if !finite(point)
            || self.image_size.0 == 0
            || self.image_size.1 == 0
            || self.widget_rect.width <= 0.0
            || self.widget_rect.height <= 0.0
        {
            return None;
        }
        Some((
            (point.0 - self.widget_rect.x) * f64::from(self.image_size.0) / self.widget_rect.width,
            (point.1 - self.widget_rect.y) * f64::from(self.image_size.1) / self.widget_rect.height,
        ))
    }

    /// 【画布导航】【原图转窗口】按point返回窗口坐标，保留越界点供正文裁切，无效原图或非有限点返回空值
    pub fn image_to_widget(self, point: Point) -> Option<Point> {
        if !finite(point) || self.image_size.0 == 0 || self.image_size.1 == 0 {
            return None;
        }
        Some((
            self.widget_rect.x + point.0 * self.widget_rect.width / f64::from(self.image_size.0),
            self.widget_rect.y + point.1 * self.widget_rect.height / f64::from(self.image_size.1),
        ))
    }
}

/// 【画布导航】【坐标检查】按point返回两个坐标是否均为有限值
pub fn finite(point: Point) -> bool {
    point.0.is_finite() && point.1.is_finite()
}
