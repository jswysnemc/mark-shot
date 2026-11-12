//! 【扫码窗口】【布局】正文与底栏随窗口尺寸调整，绘制和命中共用几何

use tiny_skia::Rect;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Control {
    Copy,
    CopyAll,
    Close,
}

pub(super) struct Layout {
    pub title: Rect,
    pub status: Rect,
    pub viewport: Rect,
    pub text: Rect,
    pub buttons: [(Control, Rect); 3],
}

/// 【扫码窗口】【矩形构造】按位置与尺寸返回有效逻辑矩形，最小边长为一像素
pub(super) fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_xywh(x, y, w.max(1.0), h.max(1.0)).unwrap()
}

/// 【扫码窗口】【命中检查】按 area 和 point 判断是否位于矩形内，返回布尔值
pub(super) fn contains(area: Rect, point: (f64, f64)) -> bool {
    point.0 >= f64::from(area.left())
        && point.0 < f64::from(area.right())
        && point.1 >= f64::from(area.top())
        && point.1 < f64::from(area.bottom())
}

impl Layout {
    /// 【扫码窗口】【尺寸适配】按 size 返回标题、结果和按钮区域，正文预留滚动条宽度
    pub fn new(size: (u32, u32)) -> Self {
        let (w, h) = (size.0 as f32, size.1 as f32);
        Self {
            title: rect(14.0, 8.0, w - 110.0, 26.0),
            status: rect(14.0, 38.0, w - 28.0, 24.0),
            viewport: rect(14.0, 72.0, w - 28.0, h - 126.0),
            text: rect(18.0, 76.0, w - 48.0, h - 134.0),
            buttons: [
                (Control::Copy, rect(14.0, h - 42.0, 132.0, 28.0)),
                (Control::CopyAll, rect(w - 154.0, h - 42.0, 140.0, 28.0)),
                (Control::Close, rect(w - 86.0, 8.0, 72.0, 26.0)),
            ],
        }
    }

    /// 【扫码窗口】【按钮命中】按 point 返回按钮身份，空白区域返回空值
    pub fn hit(&self, point: (f64, f64)) -> Option<Control> {
        self.buttons
            .iter()
            .find(|(_, area)| contains(*area, point))
            .map(|(control, _)| *control)
    }
}
