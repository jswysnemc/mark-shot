//! 【尺子】【数字条带】完整条带决定换侧、约束、刻度范围和顶部留量

use super::layout_rect::ExactRect;
use super::{
    Measurement, Point, RulerGeometry,
    geometry::{points_bounds, rect_center, rotated},
};
use crate::features::annotation_style::window::WindowUnits;
use tiny_skia::Rect;

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub paint_rect: Rect,
    pub horizontal_band: Rect,
    pub vertical_band: Rect,
    pub below: bool,
    pub right: bool,
    pub clamp: bool,
    pub unit: f32,
    pub font_pixels: f32,
    pub step_x: u32,
    pub step_y: u32,
    pub rotation: f32,
    pub allowed: Option<Rect>,
    pub window: WindowUnits,
    projected_scale: ExactRect,
}

/// 【尺子】【旋转矩形】将rect绕center旋转degrees，返回外接矩形；矩形使用当前画布坐标
pub fn rotated_rect(rect: Rect, center: Point, degrees: f32) -> Option<Rect> {
    if degrees.abs() <= f32::EPSILON {
        return Some(rect);
    }
    points_bounds(
        &[
            (rect.left(), rect.top()),
            (rect.right(), rect.top()),
            (rect.right(), rect.bottom()),
            (rect.left(), rect.bottom()),
        ]
        .map(|point| rotated(point, center, degrees)),
        f32::EPSILON,
    )
}

/// 【尺子】【单轴约束】按rect、bounds和horizontal平移矩形，返回约束后的完整尺寸，不缩小数字
fn clamped_along(rect: Rect, bounds: Rect, horizontal: bool) -> Rect {
    let (left, top) = if horizontal {
        (
            rect.left().clamp(
                bounds.left(),
                (bounds.right() - rect.width()).max(bounds.left()),
            ),
            rect.top(),
        )
    } else {
        (
            rect.left(),
            rect.top().clamp(
                bounds.top(),
                (bounds.bottom() - rect.height()).max(bounds.top()),
            ),
        )
    };
    Rect::from_xywh(left, top, rect.width(), rect.height()).unwrap_or(rect)
}

impl Layout {
    /// 【尺子】【完整布局】按measurement、width、allowed和window返回数字条带及刻度间隔，角度尺返回空值
    pub fn new(
        measurement: Measurement,
        width: f32,
        allowed: Option<Rect>,
        window: WindowUnits,
    ) -> Option<Self> {
        Self::build(measurement, width, allowed, window, None)
    }

    /// 【尺子】【当前窗口布局】按measurement、width、allowed及projection在双精度窗口坐标中决定布局，再返回稳定坐标
    pub(crate) fn in_projection(
        measurement: Measurement,
        width: f32,
        allowed: Option<Rect>,
        projection: crate::editor::canvas_navigation::transform::ViewTransform,
    ) -> Option<Self> {
        Self::build(
            measurement,
            width,
            allowed,
            WindowUnits::new(1.0 / projection.scale as f32),
            Some(projection),
        )
    }

    /// 【尺子】【布局计算】按测量、笔宽、允许范围、窗口单位和可选投影返回完整布局，窗口判断不省略平移
    fn build(
        measurement: Measurement,
        width: f32,
        allowed: Option<Rect>,
        window: WindowUnits,
        projection: Option<crate::editor::canvas_navigation::transform::ViewTransform>,
    ) -> Option<Self> {
        let RulerGeometry::Length {
            start,
            end,
            rotation,
        } = measurement.geometry
        else {
            return None;
        };
        let raw_width = (end.0 - start.0).abs();
        let raw_height = (end.1 - start.1).abs();
        let paint_rect = Rect::from_xywh(
            start.0.min(end.0),
            start.1.min(end.1),
            raw_width,
            raw_height,
        )?;
        // 1. 【尺子】【双精度条带】使用与原版相同的逐项乘除和减法，临界贴边时不合并算式
        let coordinate_scale = projection.map_or(1.0, |view| view.scale);
        let unit64 = (1.0 + (f64::from(width * measurement.pixel_scale) - 3.0) / 9.0)
            .clamp(0.75, 3.5)
            * measurement.style_scale
            * coordinate_scale;
        let unit = (unit64 / coordinate_scale) as f32;
        let paint_per_window = if projection.is_some() {
            1.0
        } else {
            f64::from(window.canvas(1.0))
        };
        let font_pixels = (13.0 * unit64 / paint_per_window).round().max(6.0) as f32;
        let x = crate::fonts::ruler::measure(
            &(raw_width * measurement.pixel_scale).round().to_string(),
            font_pixels,
        );
        let y = crate::fonts::ruler::measure(
            &(raw_height * measurement.pixel_scale).round().to_string(),
            font_pixels,
        );
        let overhang = f64::from(x.width) * paint_per_window / 2.0;
        let vertical_width = f64::from(y.width.ceil()) * paint_per_window;
        let body = ExactRect::from_rect(paint_rect).projected(projection, false);
        let horizontal = |below| ExactRect {
            left: body.left - overhang,
            top: if below {
                body.bottom() + 14.0 * unit64
            } else {
                body.top - 14.0 * unit64 - 16.0 * unit64
            },
            width: body.width + overhang * 2.0,
            height: 16.0 * unit64,
        };
        let vertical = |right| ExactRect {
            left: if right {
                body.right() + 14.0 * unit64
            } else {
                body.left - 14.0 * unit64 - vertical_width
            },
            top: body.top - 8.0 * unit64,
            width: vertical_width,
            height: body.height + 16.0 * unit64,
        };
        let mut h = horizontal(false);
        let mut v = vertical(false);
        let mut below = false;
        let mut right = false;
        let mut clamp = false;
        if let Some(bounds) = allowed {
            let bounds = ExactRect::from_rect(bounds).projected(projection, false);
            // 2. 【尺子】【完整换侧】未旋转时只判断垂直轴，旋转时判断整条条带的外接矩形
            if !h.fits(bounds, body, rotation, true)
                && horizontal(true).fits(bounds, body, rotation, true)
            {
                below = true;
                h = horizontal(true);
            }
            if !v.fits(bounds, body, rotation, false)
                && vertical(true).fits(bounds, body, rotation, false)
            {
                right = true;
                v = vertical(true);
            }
            clamp = rotation.abs() <= f32::EPSILON
                && body.right() >= bounds.left
                && body.left <= bounds.right()
                && body.bottom() >= bounds.top
                && body.top <= bounds.bottom();
            if clamp {
                h.top = h
                    .top
                    .clamp(bounds.top, (bounds.bottom() - h.height).max(bounds.top));
                v.left = v
                    .left
                    .clamp(bounds.left, (bounds.right() - v.width).max(bounds.left));
            }
        }
        let projected_scale = body.outset(13.0 * unit64).joined(h).joined(v).rotated(
            (body.left + body.width / 2.0, body.top + body.height / 2.0),
            rotation,
        );
        Some(Self {
            paint_rect,
            horizontal_band: h.projected(projection, true).rect()?,
            vertical_band: v.projected(projection, true).rect()?,
            below,
            right,
            clamp,
            unit,
            font_pixels,
            rotation,
            allowed,
            window,
            projected_scale,
            step_x: super::ticks::major_step(
                raw_width * measurement.pixel_scale,
                raw_width,
                window.canvas(x.width) + 6.0 * unit,
            ),
            step_y: super::ticks::major_step(
                raw_height * measurement.pixel_scale,
                raw_height,
                window.canvas(y.height) + 3.0 * unit,
            ),
        })
    }

    /// 【尺子】【顶部留量】无参数，返回完整条带或主刻度超出正文上边的距离，单位为画布坐标
    pub fn top_reach(self) -> f32 {
        (13.0 * self.unit).max(self.paint_rect.top() - self.horizontal_band.top())
    }

    /// 【尺子】【双精度刻度范围】无参数，返回布局判断所用的完整窗口刻度范围，说明避让不经过渲染矩形转换
    pub(crate) fn projected_scale_bounds(self) -> ExactRect {
        self.projected_scale
    }

    /// 【尺子】【完整刻度范围】无参数，返回正文、四边刻度和两条数字条带旋转后的外接矩形
    pub fn scale_bounds(self) -> Option<Rect> {
        let rect = self
            .paint_rect
            .outset(13.0 * self.unit, 13.0 * self.unit)?
            .join(&self.horizontal_band)?
            .join(&self.vertical_band)?;
        rotated_rect(rect, rect_center(self.paint_rect), self.rotation)
    }

    /// 【尺子】【单项数字】按position、text与horizontal返回与条带一致的数字矩形，允许范围内刻度只沿对应轴约束
    pub fn number(self, position: f32, text: &str, horizontal: bool) -> Option<Rect> {
        let rect = if horizontal {
            let width = self
                .window
                .canvas(crate::fonts::ruler::measure(text, self.font_pixels).width);
            Rect::from_xywh(
                position - width / 2.0,
                self.horizontal_band.top(),
                width,
                self.horizontal_band.height(),
            )?
        } else {
            Rect::from_xywh(
                self.vertical_band.left(),
                position - 8.0 * self.unit,
                self.vertical_band.width(),
                16.0 * self.unit,
            )?
        };
        Some(
            if let Some(bounds) = self.allowed
                && self.clamp
                && if horizontal {
                    position >= bounds.left() && position <= bounds.right()
                } else {
                    position >= bounds.top() && position <= bounds.bottom()
                }
            {
                clamped_along(rect, bounds, horizontal)
            } else {
                rect
            },
        )
    }
}
