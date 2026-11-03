//! 【图像采样】【冻结原图】提供轻量像素视图，屏幕绘制和导出均不读取已标注图层

use tiny_skia::{Pixmap, Rect};

use crate::editor::file_canvas::FileCanvas;
use crate::types::{Capture, Placement};

#[derive(Clone, Copy)]
enum Source<'a> {
    Image {
        pixels: &'a Pixmap,
        viewport: Rect,
    },
    Desktop {
        captures: &'a [Capture],
        placements: &'a [Placement],
    },
}

#[derive(Clone, Copy)]
pub struct FrozenImage<'a> {
    source: Source<'a>,
    factor: f32,
    offset: (f32, f32),
}

impl<'a> FrozenImage<'a> {
    /// 【图像采样】【原图网格】返回当前绘图坐标对应的原始像素范围和比例，不复制图像或读取标注
    pub fn native_grid(self) -> Option<crate::features::filter::geometry::PixelGrid> {
        let (bounds, scale, width, height) = match self.source {
            Source::Image { pixels, viewport } => (
                viewport,
                pixels.width() as f32 / viewport.width(),
                pixels.width(),
                pixels.height(),
            ),
            Source::Desktop {
                captures,
                placements,
            } => {
                let bounds = crate::utils::get_full_workspace_rect(placements)?;
                let scale = super::capture_scale(captures, placements, bounds);
                let width = (bounds.width() * scale).ceil() as u32;
                let height = (bounds.height() * scale).ceil() as u32;
                (bounds, scale, width, height)
            }
        };
        let bounds = Rect::from_ltrb(
            bounds.left() * self.factor + self.offset.0,
            bounds.top() * self.factor + self.offset.1,
            bounds.right() * self.factor + self.offset.0,
            bounds.bottom() * self.factor + self.offset.1,
        )?;
        crate::features::filter::geometry::PixelGrid::new(
            bounds,
            scale / self.factor,
            width,
            height,
        )
    }

    /// 【图像采样】【原始像素】读取当前 native_grid 对应的 x、y；本地图像直接读取，多输出按 grid 插值，未覆盖位置返回空值
    pub fn native_pixel(
        self,
        grid: crate::features::filter::geometry::PixelGrid,
        x: u32,
        y: u32,
    ) -> Option<[u8; 4]> {
        match self.source {
            Source::Image { pixels, .. } => {
                if x >= pixels.width() || y >= pixels.height() {
                    return None;
                }
                let offset = (y as usize * pixels.width() as usize + x as usize) * 4;
                Some(pixels.data()[offset..offset + 4].try_into().unwrap())
            }
            Source::Desktop { .. } => {
                self.sample(grid.point(f64::from(x) + 0.5, f64::from(y) + 0.5))
            }
        }
    }

    /// 【图像采样】【本地图像】以 pixels 和全局 viewport 创建零复制原图视图；返回采样器
    pub fn image(pixels: &'a Pixmap, viewport: Rect) -> Self {
        Self {
            source: Source::Image { pixels, viewport },
            factor: 1.0,
            offset: (0.0, 0.0),
        }
    }

    /// 【图像采样】【桌面图像】以 captures、placements 创建多输出零复制视图；返回采样器，未覆盖位置保持透明
    pub fn desktop(captures: &'a [Capture], placements: &'a [Placement]) -> Self {
        Self {
            source: Source::Desktop {
                captures,
                placements,
            },
            factor: 1.0,
            offset: (0.0, 0.0),
        }
    }

    /// 【图像采样】【编辑来源】优先使用 file 的原图，否则使用 captures、placements；返回不含持久标注的视图
    pub fn from_parts(
        file: Option<&'a FileCanvas>,
        captures: &'a [Capture],
        placements: &'a [Placement],
    ) -> Self {
        file.map_or_else(
            || Self::desktop(captures, placements),
            |file| Self::image(&file.original, file.viewport),
        )
    }

    /// 【图像采样】【导出映射】将 factor、offset 对应的绘图坐标反解到原始逻辑坐标；返回新视图，无效参数保持原视图
    pub fn mapped(self, factor: f32, offset: (f32, f32)) -> Self {
        if !factor.is_finite() || factor <= 0.0 || !offset.0.is_finite() || !offset.1.is_finite() {
            return self;
        }
        Self {
            factor,
            offset,
            ..self
        }
    }

    /// 【图像采样】【平滑像素】返回 point 对应的双线性预乘 RGBA；点在原图外或非有限时返回空值，不转换透明像素为白色
    pub fn sample(self, point: (f32, f32)) -> Option<[u8; 4]> {
        if !point.0.is_finite() || !point.1.is_finite() {
            return None;
        }
        let point = (
            (point.0 - self.offset.0) / self.factor,
            (point.1 - self.offset.1) / self.factor,
        );
        match self.source {
            Source::Image { pixels, viewport } => {
                if !contains(viewport, point) {
                    return None;
                }
                bilinear(
                    pixels,
                    (
                        f64::from(point.0 - viewport.left()) * f64::from(pixels.width())
                            / f64::from(viewport.width()),
                        f64::from(point.1 - viewport.top()) * f64::from(pixels.height())
                            / f64::from(viewport.height()),
                    ),
                )
            }
            Source::Desktop {
                captures,
                placements,
            } => captures
                .iter()
                .zip(placements)
                .rev()
                .find_map(|(capture, placement)| {
                    let bounds = Rect::from_xywh(
                        placement.position.0 as f32,
                        placement.position.1 as f32,
                        placement.size.0 as f32,
                        placement.size.1 as f32,
                    )?;
                    if !contains(bounds, point) {
                        return None;
                    }
                    let local = (
                        f64::from(point.0 - bounds.left()),
                        f64::from(point.1 - bounds.top()),
                    );
                    bilinear(&capture.pixmap, capture.to_native(local))
                }),
        }
    }
}

/// 【图像采样】【范围检查】返回 point 是否位于 bounds 的左闭右开范围，避免读取相邻输出的第一列
fn contains(bounds: Rect, point: (f32, f32)) -> bool {
    point.0 >= bounds.left()
        && point.0 < bounds.right()
        && point.1 >= bounds.top()
        && point.1 < bounds.bottom()
}

/// 【图像采样】【双线性插值】以 point 的原图像素中心坐标读取 pixels；返回预乘 RGBA，边界使用最近有效像素
fn bilinear(pixels: &Pixmap, point: (f64, f64)) -> Option<[u8; 4]> {
    if !point.0.is_finite() || !point.1.is_finite() {
        return None;
    }
    let x = (point.0 - 0.5).clamp(0.0, f64::from(pixels.width() - 1));
    let y = (point.1 - 0.5).clamp(0.0, f64::from(pixels.height() - 1));
    let (x0, y0) = (x.floor() as u32, y.floor() as u32);
    let (x1, y1) = (
        (x0 + 1).min(pixels.width() - 1),
        (y0 + 1).min(pixels.height() - 1),
    );
    let (dx, dy) = (x - f64::from(x0), y - f64::from(y0));
    let index = |x: u32, y: u32| (y as usize * pixels.width() as usize + x as usize) * 4;
    let offsets = [index(x0, y0), index(x1, y0), index(x0, y1), index(x1, y1)];
    let data = pixels.data();
    // 1. 【图像采样】【双线性插值】颜色与透明度在预乘空间内插值，防止透明边缘出现黑色或彩色光晕
    Some(std::array::from_fn(|channel| {
        let top = f64::from(data[offsets[0] + channel]) * (1.0 - dx)
            + f64::from(data[offsets[1] + channel]) * dx;
        let bottom = f64::from(data[offsets[2] + channel]) * (1.0 - dx)
            + f64::from(data[offsets[3] + channel]) * dx;
        (top * (1.0 - dy) + bottom * dy).round().clamp(0.0, 255.0) as u8
    }))
}
