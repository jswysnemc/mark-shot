//! 【滤镜】【原图分块】缓存键只依赖冻结原图网格、对齐区域、模式和强度，不依赖先前标注

use shot_image::{ImageView, blur, filters, pixelate};
use tiny_skia::{IntRect, Pixmap};

use super::{FilterMode, RegionFilter, geometry::PixelGrid};
use crate::renderer::FrozenImage;

pub const TILE_PIXELS: usize = 512;
pub const MAX_CACHED_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilterKey {
    pub grid: PixelGrid,
    pub region: IntRect,
    pub mode: FilterMode,
    pub strength: f32,
}

impl FilterKey {
    /// 【滤镜】【缓存参数】以 image、filter 创建冻结原图缓存键；旋转不改变源像素，空区域返回空值
    pub fn new(image: FrozenImage<'_>, filter: RegionFilter) -> Option<Self> {
        let grid = image.native_grid()?;
        Some(Self {
            grid,
            region: grid.region(filter)?,
            mode: filter.mode,
            strength: filter.strength,
        })
    }

    /// 【滤镜】【缓存容量】返回完整区域的字节数，乘法溢出返回空值
    pub fn bytes(self) -> Option<usize> {
        (self.region.width() as usize)
            .checked_mul(self.region.height() as usize)?
            .checked_mul(4)
    }

    /// 【滤镜】【原图块列表】返回 region 内不重叠的五百一十二像素核心块，不分配完整图像
    pub fn tiles(self) -> impl Iterator<Item = IntRect> {
        (0..self.region.height())
            .step_by(TILE_PIXELS)
            .flat_map(move |y| {
                (0..self.region.width()).step_by(TILE_PIXELS).map(move |x| {
                    IntRect::from_xywh(
                        self.region.x() + x as i32,
                        self.region.y() + y as i32,
                        (self.region.width() - x).min(TILE_PIXELS as u32),
                        (self.region.height() - y).min(TILE_PIXELS as u32),
                    )
                    .unwrap()
                })
            })
    }
}

pub struct Raster {
    pub key: FilterKey,
    pub pixels: Pixmap,
}

pub struct Tile {
    pub at: IntRect,
    pub pixels: Pixmap,
}

#[derive(Clone, Copy)]
pub struct PixelView<'a> {
    pub at: IntRect,
    pub pixels: &'a Pixmap,
}

impl Raster {
    /// 【滤镜】【完整区域缓存】以 image、key 分块生成最多四 MiB 的区域；过大或分配失败返回空值
    pub fn new(image: FrozenImage<'_>, key: FilterKey) -> Option<Self> {
        if key.bytes()? > MAX_CACHED_BYTES {
            return None;
        }
        let mut pixels = Pixmap::new(key.region.width(), key.region.height())?;
        for core in key.tiles() {
            let tile = render_tile(image, key, core)?;
            let local = core.translate(-tile.at.x(), -tile.at.y())?;
            let piece = tile.pixels.clone_rect(local)?;
            crate::renderer::paste(
                &mut pixels,
                &piece,
                core.x() - key.region.x(),
                core.y() - key.region.y(),
            );
        }
        Some(Self { key, pixels })
    }

    /// 【滤镜】【缓存像素视图】返回区域像素的零复制视图，不复制整个缓存
    pub fn view(&self) -> PixelView<'_> {
        PixelView {
            at: self.key.region,
            pixels: &self.pixels,
        }
    }
}

impl Tile {
    /// 【滤镜】【块像素视图】返回带一像素插值留量的零复制视图
    pub fn view(&self) -> PixelView<'_> {
        PixelView {
            at: self.at,
            pixels: &self.pixels,
        }
    }
}

impl PixelView<'_> {
    /// 【滤镜】【块内插值】读取原图连续 point；nearest 控制像素化硬边，返回预乘 RGBA，边界夹取最近像素
    pub fn sample(self, point: (f64, f64), nearest: bool) -> [u8; 4] {
        let x = point.0 - f64::from(self.at.x());
        let y = point.1 - f64::from(self.at.y());
        let pixel = |x: u32, y: u32| {
            let offset = (y as usize * self.pixels.width() as usize + x as usize) * 4;
            &self.pixels.data()[offset..offset + 4]
        };
        if nearest {
            return pixel(
                x.floor().clamp(0.0, f64::from(self.pixels.width() - 1)) as u32,
                y.floor().clamp(0.0, f64::from(self.pixels.height() - 1)) as u32,
            )
            .try_into()
            .unwrap();
        }
        let x = (x - 0.5).clamp(0.0, f64::from(self.pixels.width() - 1));
        let y = (y - 0.5).clamp(0.0, f64::from(self.pixels.height() - 1));
        let (x0, y0) = (x.floor() as u32, y.floor() as u32);
        let (x1, y1) = (
            (x0 + 1).min(self.pixels.width() - 1),
            (y0 + 1).min(self.pixels.height() - 1),
        );
        let (dx, dy) = (x.fract(), y.fract());
        std::array::from_fn(|channel| {
            let top = f64::from(pixel(x0, y0)[channel]) * (1.0 - dx)
                + f64::from(pixel(x1, y0)[channel]) * dx;
            let bottom = f64::from(pixel(x0, y1)[channel]) * (1.0 - dx)
                + f64::from(pixel(x1, y1)[channel]) * dx;
            (top * (1.0 - dy) + bottom * dy).round().clamp(0.0, 255.0) as u8
        })
    }
}

/// 【滤镜】【原图块生成】按 image、key 为 core 生成滤镜块和一像素插值留量；只在模糊时读取区域外像素
pub fn render_tile(image: FrozenImage<'_>, key: FilterKey, core: IntRect) -> Option<Tile> {
    if core.width() > TILE_PIXELS as u32 || core.height() > TILE_PIXELS as u32 {
        return None;
    }
    let at = core.make_outset(1, 1)?.intersect(&key.region)?;
    let source = match key.mode {
        FilterMode::Blur => {
            let radius = filters::blur_radius(f64::from(key.strength));
            at.make_outset(radius * 2, radius * 2)?
                .intersect(&key.grid.pixels())?
        }
        FilterMode::Pixelate => cell_bounds(at, key.region, key.strength.round() as i32)?,
        _ => at,
    };
    let mut values =
        shot_image::buffer::allocate(source.width() as usize * source.height() as usize).ok()?;
    // 1. 【滤镜】【原图块生成】直接读取冻结原图，先前标注和其他滤镜不会成为输入
    for y in 0..source.height() {
        for x in 0..source.width() {
            let pixel = image
                .native_pixel(key.grid, source.x() as u32 + x, source.y() as u32 + y)
                .unwrap_or([0; 4]);
            let value = if matches!(
                key.mode,
                FilterMode::Grayscale | FilterMode::Invert | FilterMode::Brighten
            ) {
                let straight = straight_argb(pixel);
                let filter = match key.mode {
                    FilterMode::Grayscale => filters::Filter::Grayscale,
                    FilterMode::Invert => filters::Filter::Invert,
                    _ => filters::Filter::Brighten(filters::brightness_amount(f64::from(
                        key.strength,
                    ))),
                };
                filters::transform(straight, filter)
            } else {
                packed_argb(pixel)
            };
            values[y as usize * source.width() as usize + x as usize] = value;
        }
    }
    // 2. 【滤镜】【原图块生成】模糊只保留一份输入与一份暂存，像素化网格锚定整个区域左上角
    let width = source.width() as i32;
    let height = source.height() as i32;
    let filtered = match key.mode {
        FilterMode::Blur => blur::gaussian_owned(
            values,
            width,
            height,
            filters::blur_radius(f64::from(key.strength)),
        )
        .ok()?,
        FilterMode::Pixelate => pixelate::apply(
            ImageView::new(&values, width, height, width as usize).ok()?,
            key.strength.round() as i32,
        )
        .ok()?,
        _ => shot_image::Image::from_pixels(width, height, values).ok()?,
    };
    let mut pixels = Pixmap::new(at.width(), at.height())?;
    for y in 0..at.height() {
        let row = filtered.row(y as i32 + at.y() - source.y());
        for x in 0..at.width() {
            let value = row[(x as i32 + at.x() - source.x()) as usize];
            let offset = (y as usize * at.width() as usize + x as usize) * 4;
            pixels.data_mut()[offset..offset + 4].copy_from_slice(&[
                ((value >> 16) & 255) as u8,
                ((value >> 8) & 255) as u8,
                (value & 255) as u8,
                (value >> 24) as u8,
            ]);
        }
    }
    Some(Tile { at, pixels })
}

/// 【滤镜】【网格范围】把 at 对齐到 region 左上角的 block 网格；返回完整有效块，不引入区域外像素
fn cell_bounds(at: IntRect, region: IntRect, block: i32) -> Option<IntRect> {
    let block = block.clamp(2, 96);
    let left = region.x() + (at.x() - region.x()).div_euclid(block) * block;
    let top = region.y() + (at.y() - region.y()).div_euclid(block) * block;
    let right = region.x() + (at.right() - region.x() + block - 1).div_euclid(block) * block;
    let bottom = region.y() + (at.bottom() - region.y() + block - 1).div_euclid(block) * block;
    IntRect::from_ltrb(
        left,
        top,
        right.min(region.right()),
        bottom.min(region.bottom()),
    )
}

/// 【滤镜】【预乘像素打包】把预乘 RGBA pixel 转为 ARGB 整数，不转换颜色空间
fn packed_argb(pixel: [u8; 4]) -> u32 {
    (u32::from(pixel[3]) << 24)
        | (u32::from(pixel[0]) << 16)
        | (u32::from(pixel[1]) << 8)
        | u32::from(pixel[2])
}

/// 【滤镜】【直通像素打包】把预乘 RGBA pixel 反预乘后转为 ARGB；完全透明像素返回零
fn straight_argb(pixel: [u8; 4]) -> u32 {
    let alpha = u32::from(pixel[3]);
    if alpha == 0 {
        return 0;
    }
    let channel = |value: u8| ((u32::from(value) * 255 + alpha / 2) / alpha).min(255);
    (alpha << 24) | (channel(pixel[0]) << 16) | (channel(pixel[1]) << 8) | channel(pixel[2])
}
