//! 【文字识别】【原图交互】按实际 Qt 整数几何保存指针中心缩放、左键平移及复位状态

use crate::backend::window::WindowCursor;

#[cfg(test)]
mod tests;

pub(super) struct SourcePreview {
    pub zoom: f64,
    pub pan: (f64, f64),
    pub dragging: bool,
    anchor: (f64, f64),
    pan_anchor: (f64, f64),
    pub raster: Option<tiny_skia::Pixmap>,
}

impl Default for SourcePreview {
    /// 【文字识别】【原图初始状态】创建适配视区且未拖动的状态，无参数，返回原图交互状态
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: (0.0, 0.0),
            dragging: false,
            anchor: (0.0, 0.0),
            pan_anchor: (0.0, 0.0),
            raster: None,
        }
    }
}

impl SourcePreview {
    /// 【文字识别】【适配尺寸】按 viewport 与 image 的整数比例计算尺寸，再应用 zoom，返回宽高；无效视区返回空值
    fn fitted(&self, viewport: (i32, i32), image: (u32, u32)) -> Option<(i32, i32)> {
        let (width, height) = (i64::from(viewport.0 - 8), i64::from(viewport.1 - 8));
        let (iw, ih) = (i64::from(image.0), i64::from(image.1));
        if width <= 0 || height <= 0 || iw == 0 || ih == 0 {
            return None;
        }
        let (fw, fh) = if iw * height > ih * width {
            (width, ih * width / iw)
        } else {
            (iw * height / ih, height)
        };
        Some((
            ((fw as f64 * self.zoom).round() as i32).max(1),
            ((fh as f64 * self.zoom).round() as i32).max(1),
        ))
    }

    /// 【文字识别】【原图矩形】按 viewport、image 返回视区本地整数矩形；中心遵循 QRect 包含右下边界的计算规则
    pub fn rect(&self, viewport: (i32, i32), image: (u32, u32)) -> Option<[i32; 4]> {
        let (width, height) = self.fitted(viewport, image)?;
        let center = (
            4 + (viewport.0 - 9) / 2 + self.pan.0.round() as i32,
            4 + (viewport.1 - 9) / 2 + self.pan.1.round() as i32,
        );
        Some([
            center.0 - (width - 1) / 2,
            center.1 - (height - 1) / 2,
            width,
            height,
        ])
    }

    /// 【文字识别】【可见边界】用 viewport、image 限制平移，至少保留 48 个逻辑像素；返回值为空，不修改缩放
    fn clamp_pan(&mut self, viewport: (i32, i32), image: (u32, u32)) {
        if self.zoom <= 1.01 {
            self.pan = (0.0, 0.0);
            return;
        }
        let Some([mut x, mut y, width, height]) = self.rect(viewport, image) else {
            return;
        };
        if x + width - 1 < 52 {
            x = 53 - width;
        }
        if x > viewport.0 - 53 {
            x = viewport.0 - 53;
        }
        if y + height - 1 < 52 {
            y = 53 - height;
        }
        if y > viewport.1 - 53 {
            y = viewport.1 - 53;
        }
        self.pan = (
            f64::from(x + (width - 1) / 2 - (4 + (viewport.0 - 9) / 2)),
            f64::from(y + (height - 1) / 2 - (4 + (viewport.1 - 9) / 2)),
        );
    }

    /// 【文字识别】【滚轮缩放】按 angle 的符号围绕 position 缩放 viewport 中的 image；返回是否接受滚轮，零角度不改变状态
    pub fn wheel(
        &mut self,
        viewport: (i32, i32),
        image: (u32, u32),
        position: (f64, f64),
        angle: f64,
    ) -> bool {
        if !angle.is_finite() || angle == 0.0 {
            return false;
        }
        let Some([x, y, width, height]) = self.rect(viewport, image) else {
            return false;
        };
        // 1. 【文字识别】【指针锚点】保存指针在旧图中的比例，不以视区中心代替指针
        let (fx, fy) = (
            (position.0 - f64::from(x)) / f64::from(width),
            (position.1 - f64::from(y)) / f64::from(height),
        );
        self.zoom = (self.zoom * if angle > 0.0 { 1.12 } else { 1.0 / 1.12 }).clamp(1.0, 6.0);
        if self.zoom <= 1.01 {
            self.zoom = 1.0;
            self.pan = (0.0, 0.0);
        } else if let Some((width, height)) = self.fitted(viewport, image) {
            // 2. 【文字识别】【缩放偏移】按新尺寸还原锚点，再应用实际 Qt 可见边界
            self.pan = (
                position.0 - fx * f64::from(width) + f64::from(width) / 2.0
                    - f64::from(4 + (viewport.0 - 9) / 2),
                position.1 - fy * f64::from(height) + f64::from(height) / 2.0
                    - f64::from(4 + (viewport.1 - 9) / 2),
            );
            self.clamp_pan(viewport, image);
        }
        true
    }

    /// 【文字识别】【左键平移开始】以 position 保存指针与偏移锚点，返回是否开始拖动；适配大小不启动平移
    pub fn press(&mut self, position: (f64, f64)) -> bool {
        if self.zoom <= 1.01 {
            return false;
        }
        self.dragging = true;
        self.anchor = position;
        self.pan_anchor = self.pan;
        true
    }

    /// 【文字识别】【左键平移更新】用 viewport、image 限制 position 对应偏移，返回是否正在拖动；右键和未抓取移动不调用此事务
    pub fn motion(
        &mut self,
        viewport: (i32, i32),
        image: (u32, u32),
        position: (f64, f64),
    ) -> bool {
        if !self.dragging {
            return false;
        }
        self.pan = (
            self.pan_anchor.0 + position.0 - self.anchor.0,
            self.pan_anchor.1 + position.1 - self.anchor.1,
        );
        self.clamp_pan(viewport, image);
        true
    }

    /// 【文字识别】【平移结束】取消当前抓取并保留缩放与偏移，无参数，返回是否结束了拖动
    pub fn release(&mut self) -> bool {
        std::mem::take(&mut self.dragging)
    }

    /// 【文字识别】【双击复位】恢复适配大小、居中位置并清除拖动，无参数、无返回值
    pub fn reset(&mut self) {
        self.zoom = 1.0;
        self.pan = (0.0, 0.0);
        self.dragging = false;
    }

    /// 【文字识别】【原图光标】按当前缩放和抓取返回普通、张手或握手光标，无参数
    pub fn cursor(&self) -> WindowCursor {
        if self.dragging {
            WindowCursor::Grabbing
        } else if self.zoom > 1.01 {
            WindowCursor::Grab
        } else {
            WindowCursor::Default
        }
    }
}
