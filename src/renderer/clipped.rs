//! 【局部绘制】【像素范围】动态元素只绘入已恢复区域，窄缓冲保留当前背景及原始像素合成顺序

use tiny_skia::{Pixmap, Rect};

use super::{Painter, rect_bounds, scale_rect};

impl Painter<'_> {
    /// 【局部绘制】【场景裁切】按 area 场景逻辑坐标限制 draw，返回值为空；嵌套缓冲先扣除当前原点，不改变 clipped 的本地坐标约定
    pub fn clipped_scene(&mut self, area: Option<Rect>, draw: impl FnOnce(&mut Painter<'_>)) {
        let Some(area) = area else {
            self.clipped(None, draw);
            return;
        };
        let origin = self.origin();
        let Some(local) = Rect::from_xywh(
            area.x() - origin.0,
            area.y() - origin.1,
            area.width(),
            area.height(),
        ) else {
            return;
        };
        self.clipped(Some(local), draw);
    }

    /// 【局部绘制】【范围回调】用 dirty 本地逻辑区域限制 draw，返回值为空；整屏损坏直接绘制，部分损坏只分配范围缓冲
    pub fn clipped(&mut self, dirty: Option<Rect>, draw: impl FnOnce(&mut Painter<'_>)) {
        let Some(area) = dirty else {
            draw(self);
            return;
        };
        let Some(pixels) = scale_rect(&area, self.scale()) else {
            return;
        };
        let Some((x, y, width, height)) =
            rect_bounds(&pixels, self.canvas.width(), self.canvas.height())
        else {
            return;
        };
        // 1. 【局部绘制】【整屏复用】所有像素已经恢复时直接绘制，避免释放后分配器仍保留第二份整屏缓冲
        if x == 0 && y == 0 && width == self.canvas.width() && height == self.canvas.height() {
            draw(self);
            return;
        }
        let Some(mut local) = Pixmap::new(width, height) else {
            return;
        };
        let stride = self.canvas.width() as usize * 4;
        let local_stride = width as usize * 4;
        // 2. 【局部绘制】【原像素保留】直接复制背景，避免透明中间层产生第二次预乘与舍入
        for row in 0..height as usize {
            let start = (y as usize + row) * stride + x as usize * 4;
            local.data_mut()[row * local_stride..(row + 1) * local_stride]
                .copy_from_slice(&self.canvas.data()[start..start + local_stride]);
        }
        let origin = self.origin();
        let scale = self.scale();
        draw(
            &mut self
                .sub(&mut local)
                .with_origin((origin.0 + x as f32 / scale, origin.1 + y as f32 / scale)),
        );
        // 3. 【局部绘制】【单次覆盖】只替换当前损坏范围，区域外像素完全不参与重复合成
        for row in 0..height as usize {
            let start = (y as usize + row) * stride + x as usize * 4;
            crate::utils::transparent_pixmap::copy_changed(
                &mut self.canvas.data_mut()[start..start + local_stride],
                &local.data()[row * local_stride..(row + 1) * local_stride],
            );
        }
    }
}
