//! 【画布导航】【场景映射】统一屏幕输入、正文绘制和损坏区域的等比变换

use tiny_skia::Rect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewTransform {
    pub scale: f64,
    pub translation: (f64, f64),
}

impl Default for ViewTransform {
    /// 【画布导航】【恒等构造】无参数，返回不改变稳定坐标的视图
    fn default() -> Self {
        Self {
            scale: 1.0,
            translation: (0.0, 0.0),
        }
    }
}

impl ViewTransform {
    /// 【画布导航】【窗口映射】按point返回稳定画布点对应的全局屏幕点，不裁切越界位置
    pub fn to_screen(self, point: (f64, f64)) -> (f64, f64) {
        (
            point.0 * self.scale + self.translation.0,
            point.1 * self.scale + self.translation.1,
        )
    }

    /// 【画布导航】【画布映射】按point返回全局屏幕点对应的稳定画布点，不改变UI指针
    pub fn to_canvas(self, point: (f64, f64)) -> (f64, f64) {
        (
            (point.0 - self.translation.0) / self.scale,
            (point.1 - self.translation.1) / self.scale,
        )
    }

    /// 【画布导航】【矩形映射】按rect返回完整全局屏幕边界，空矩形返回空值
    pub fn rect(self, rect: Rect) -> Option<Rect> {
        let start = self.to_screen((f64::from(rect.left()), f64::from(rect.top())));
        let end = self.to_screen((f64::from(rect.right()), f64::from(rect.bottom())));
        Rect::from_ltrb(start.0 as f32, start.1 as f32, end.0 as f32, end.1 as f32)
    }

    /// 【画布导航】【输出原点】按offset返回输出左上角对应的稳定画布坐标，供组合设备比例绘制
    pub fn origin(self, offset: (f32, f32)) -> (f32, f32) {
        let point = self.to_canvas((f64::from(offset.0), f64::from(offset.1)));
        (point.0 as f32, point.1 as f32)
    }

    /// 【画布导航】【恒等判断】无参数，返回是否可以直接复用稳定图层缓存
    pub fn is_identity(self) -> bool {
        self == Self::default()
    }
}
