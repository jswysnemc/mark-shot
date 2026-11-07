use super::pixels;

/// 物理像素空间内的圆角裁切，保留旧实现对捕获窗口边缘的两像素内缩。
pub(super) struct RoundedMask {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    radius: f64,
}

impl RoundedMask {
    /// 根据 width/height 和 corner_radius 建立遮罩，返回不持有图像内存的几何对象。
    pub(super) fn new(width: i32, height: i32, corner_radius: f64) -> Self {
        let inset = if corner_radius > 0.0 && width > 4 && height > 4 {
            2.0
        } else {
            0.0
        };
        Self {
            left: inset,
            top: inset,
            right: f64::from(width) - inset,
            bottom: f64::from(height) - inset,
            radius: (corner_radius - inset)
                .max(0.0)
                .min((f64::from(width.min(height)) - 2.0 * inset) / 2.0),
        }
    }

    /// 判断采样坐标 x/y 是否位于圆角内，返回布尔结果。
    fn contains(&self, x: f64, y: f64) -> bool {
        if x < self.left || x > self.right || y < self.top || y > self.bottom {
            return false;
        }
        let center_x = x.clamp(self.left + self.radius, self.right - self.radius);
        let center_y = y.clamp(self.top + self.radius, self.bottom - self.radius);
        (x - center_x).powi(2) + (y - center_y).powi(2) <= self.radius.powi(2)
    }

    /// 计算像素 x/y 的覆盖率，返回 0 至 255；只有圆角边缘执行 8 倍超采样。
    fn coverage(&self, x: i32, y: i32) -> u32 {
        let x = f64::from(x);
        let y = f64::from(y);
        if x < self.left || x >= self.right || y < self.top || y >= self.bottom {
            return 0;
        }
        if self.radius == 0.0
            || (x >= self.left + self.radius && x + 1.0 <= self.right - self.radius)
            || (y >= self.top + self.radius && y + 1.0 <= self.bottom - self.radius)
        {
            return 255;
        }
        if [(x, y), (x + 1.0, y), (x, y + 1.0), (x + 1.0, y + 1.0)]
            .into_iter()
            .all(|(x, y)| self.contains(x, y))
        {
            return 255;
        }
        let mut covered = 0;
        for dy in 0..8 {
            for dx in 0..8 {
                covered += u32::from(self.contains(
                    x + (f64::from(dx) + 0.5) / 8.0,
                    y + (f64::from(dy) + 0.5) / 8.0,
                ));
            }
        }
        (covered * 255 + 32) / 64
    }

    /// 将像素位置 x/y 的圆角覆盖率应用到预乘 pixel，返回裁切后的像素。
    pub(super) fn apply(&self, pixel: u32, x: i32, y: i32) -> u32 {
        pixels::scale(pixel, self.coverage(x, y))
    }
}
