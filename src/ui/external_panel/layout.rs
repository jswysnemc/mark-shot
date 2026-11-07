//! 【外部动作面板】【几何规则】原版八像素内边距、四像素标题间隔与三百六十像素列表上限

use super::state::Kind;
use tiny_skia::Rect;

pub const ROW: f32 = 40.0;

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub size: (f32, f32),
    pub title: Rect,
    pub list: Rect,
}

impl Layout {
    /// 【外部动作面板】【分区尺寸】按 kind、count、text_width、message_height 与 bounds 返回原版范围内的自适应几何
    pub fn new(
        kind: Kind,
        count: usize,
        text_width: f32,
        message_height: f32,
        bounds: (f32, f32),
    ) -> Self {
        let (minimum, maximum) = match kind {
            Kind::OpenWith => (280.0, 340.0),
            Kind::Extensions => (300.0, 380.0),
        };
        let width = (text_width + if kind == Kind::OpenWith { 62.0 } else { 36.0 })
            .clamp(minimum, maximum)
            .min((bounds.0 - 16.0).max(48.0));
        let list_height = if count == 0 {
            message_height.max(36.0)
        } else {
            (count as f32 * 36.0).clamp(48.0, 360.0)
        };
        let height = (37.0 + list_height)
            .clamp(80.0, 540.0)
            .min((bounds.1 - 16.0).max(48.0));
        Self {
            size: (width, height),
            title: Rect::from_xywh(8.0, 8.0, width - 16.0, 17.0).unwrap(),
            list: Rect::from_xywh(8.0, 29.0, width - 16.0, (height - 37.0).max(1.0)).unwrap(),
        }
    }

    /// 【外部动作面板】【工具栏锚定】按 toolbar 和 bounds 返回左侧优先位置，横向工具栏改为上下避让并限制到窗口内
    pub fn position(self, toolbar: Rect, bounds: (f32, f32)) -> (f32, f32) {
        let (width, height) = self.size;
        let (mut x, mut y) = (toolbar.left() - width - 8.0, toolbar.top());
        if toolbar.width() > toolbar.height() {
            x = toolbar.right() - width;
            y = if toolbar.top() < bounds.1 / 2.0 {
                toolbar.bottom() + 8.0
            } else {
                toolbar.top() - height - 8.0
            };
        } else if x < 8.0 {
            x = toolbar.right() + 8.0;
        }
        (
            x.floor().clamp(8.0, (bounds.0 - width - 8.0).max(8.0)),
            y.floor().clamp(8.0, (bounds.1 - height - 8.0).max(8.0)),
        )
    }
}
