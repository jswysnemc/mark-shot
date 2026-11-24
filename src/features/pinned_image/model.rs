//! 【贴图】【图像状态】保存原始像素，显示缩放不改变复制、编辑或保存分辨率

use crate::AppResult;
use tiny_skia::{IntSize, Pixmap};

pub struct PinnedImage {
    pub image: Pixmap,
    pub density: f32,
    pub zoom: f32,
    display_size: (u32, u32),
    pub always_on_top: bool,
}

impl PinnedImage {
    /// 【贴图】【图像读取】解码 bytes 并按 density 设置逻辑显示大小；非法比例和过大图像返回错误
    pub fn decode(bytes: &[u8], density: f32) -> AppResult<Self> {
        if !density.is_finite() || density <= 0.0 {
            return Err("invalid pin scale".into());
        }
        let mut decoder =
            image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(256 * 1024 * 1024);
        limits.max_image_width = Some(32768);
        limits.max_image_height = Some(32768);
        decoder.limits(limits);
        let rgba = decoder.decode()?.into_rgba8();
        let (width, height) = rgba.dimensions();
        if u64::from(width) * u64::from(height) > 64 * 1024 * 1024 {
            return Err("pin image exceeds the pixel limit".into());
        }
        let mut data = rgba.into_raw();
        crate::utils::premultiply(&mut data);
        let image = Pixmap::from_vec(
            data,
            IntSize::from_wh(width, height).ok_or("empty pin image")?,
        )
        .ok_or("invalid pin image")?;
        let display_size = (
            (width as f32 / density).round().max(1.0) as u32,
            (height as f32 / density).round().max(1.0) as u32,
        );
        Ok(Self {
            image,
            density,
            zoom: 1.0,
            display_size,
            always_on_top: true,
        })
    }

    /// 【贴图】【显示大小】返回当前原图像素换算后的逻辑尺寸，至少保留一个像素
    pub fn size(&self) -> (u32, u32) {
        self.display_size
    }

    /// 【贴图】【缩放约束】将 requested 限制到合理缩放和 bounds 的屏幕范围；返回显示尺寸是否变化
    pub fn set_zoom(&mut self, requested: f32, bounds: (u32, u32)) -> bool {
        if !requested.is_finite() || requested <= 0.0 {
            return false;
        }
        let previous = self.size();
        let fit = (bounds.0 as f32 * self.density / self.image.width() as f32)
            .min(bounds.1 as f32 * self.density / self.image.height() as f32)
            .max(0.001);
        self.zoom = requested.clamp(0.1_f32.min(fit), 6.0_f32.min(fit));
        self.display_size = (
            (self.image.width() as f32 / self.density * self.zoom)
                .round()
                .max(1.0) as u32,
            (self.image.height() as f32 / self.density * self.zoom)
                .round()
                .max(1.0) as u32,
        );
        previous != self.size()
    }

    /// 【贴图】【预览缩放】按 requested 设置原版滚轮与菜单倍率；接近原始倍率并跨越时吸附到 100%，限制到 0.1–6 倍和最小二十四像素，不限制屏幕
    pub fn scale(&mut self, requested: f32) -> bool {
        if !requested.is_finite() || requested <= 0.0 {
            return false;
        }
        let previous = self.size();
        let requested = requested.clamp(0.1, 6.0);
        // 1. 【贴图】【百倍吸附】整数像素量化会跳过原始宽度；仅本步落在 100% 附近时对齐，避免菜单和脚本的大幅跳变被拉回
        let zoom = if (requested - 1.0).abs() <= 0.2 && (self.zoom - 1.0) * (requested - 1.0) < 0.0
        {
            1.0
        } else {
            requested
        };
        let width = (self.image.width() as f32 / self.density).round().max(1.0);
        let height = (self.image.height() as f32 / self.density).round().max(1.0);
        self.set_size((
            (width * zoom).round().max(24.0) as u32,
            (height * zoom).round().max(24.0) as u32,
        ));
        previous != self.size()
    }

    /// 【贴图】【拖动尺寸】保存 size 对应完整显示大小并同步后续倍率；不修改原图像素或密度
    pub fn set_size(&mut self, size: (u32, u32)) {
        self.display_size = (size.0.max(1), size.1.max(1));
        self.zoom = self.display_size.0 as f32
            / (self.image.width() as f32 / self.density).round().max(1.0);
    }

    /// 【贴图】【无损旋转】按 clockwise 旋转九十度并应用原版倍率约束；返回成功状态，不改变像素通道与透明度
    pub fn rotate(&mut self, clockwise: bool) -> AppResult {
        let (width, height) = (self.image.width() as usize, self.image.height() as usize);
        let mut data = vec![0; width * height * 4];
        for y in 0..height {
            for x in 0..width {
                let (nx, ny) = if clockwise {
                    (height - 1 - y, x)
                } else {
                    (y, width - 1 - x)
                };
                data[(ny * height + nx) * 4..][..4]
                    .copy_from_slice(&self.image.data()[(y * width + x) * 4..][..4]);
            }
        }
        self.image = Pixmap::from_vec(
            data,
            IntSize::from_wh(height as u32, width as u32).ok_or("empty image")?,
        )
        .ok_or("cannot rotate image")?;
        self.display_size = (self.display_size.1, self.display_size.0);
        self.scale(self.zoom);
        Ok(())
    }

    /// 【贴图】【PNG输出】返回当前旋转后原图的 PNG，不添加显示边框、不应用缩放
    pub fn png(&self) -> AppResult<Vec<u8>> {
        // 1. 【贴图】【原图编码】使用截图相同的透明颜色舍入，避免复制、保存和再次编辑改变交接像素
        Ok(crate::utils::encode_png_with_alpha(self.image.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【贴图测试】【图像构造】按 width、height 创建纯色原图，返回与真实解码一致的显示状态
    fn image(width: u32, height: u32) -> PinnedImage {
        let rgba = vec![255; (width * height * 4) as usize];
        PinnedImage::decode(&crate::utils::encode_rgba(&rgba, width, height), 1.0).unwrap()
    }

    /// 【贴图测试】【百倍吸附】验证跨越原始尺寸时对齐到 100%，不再停留在量化后的 101%
    #[test]
    fn zoom_crossing_the_original_size_snaps_to_one_hundred_percent() {
        let mut image = image(174, 100);
        let original = image.size();
        image.scale(0.93);
        assert_ne!(image.zoom, 1.0);
        image.scale(image.zoom * 1.08);
        assert_eq!(image.size(), original);
        assert_eq!(image.zoom, 1.0);
        // 1. 【贴图测试】【反向跨越】从原始尺寸以上缩小同样吸附，保证两个方向都能回到 100%
        image.scale(1.08);
        image.scale(image.zoom / 1.08 * 0.99);
        assert_eq!(image.size(), original);
        assert_eq!(image.zoom, 1.0);
    }

    /// 【贴图测试】【大幅跳变】验证远离原始倍率的一次性大幅缩放不会被吸附回 100%
    #[test]
    fn large_zoom_steps_do_not_snap_to_the_original_size() {
        let mut image = image(240, 120);
        image.set_zoom(1.0, (120, 60));
        assert_eq!(image.size(), (120, 60));
        image.scale(100.0);
        assert_eq!(image.size(), (1440, 720));
    }

    /// 【贴图测试】【正常步进】验证未跨越原始尺寸时仍按倍率缩放，不被吸附干扰
    #[test]
    fn zoom_steps_away_from_the_original_size_are_not_snapped() {
        let mut image = image(174, 100);
        let original = image.size();
        image.scale(0.93);
        assert!(image.zoom < 1.0);
        image.scale(image.zoom * 0.9);
        assert!(image.zoom < 0.9);
        assert!(image.size().0 < original.0);
    }
}
