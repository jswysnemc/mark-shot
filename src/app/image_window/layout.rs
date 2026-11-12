//! 【图片窗口】【布局边界】窗口优先容纳完整工具栏，原图适配其下方的可用画布

use crate::{
    config::toolbar_layout::Profile,
    ui::{
        image_window::{INSET, TITLE_HEIGHT},
        toolbar::{Orientation, Toolbar},
    },
};
use tiny_skia::Rect;

/// 【图片窗口】【工具栏测量】无参数，返回当前配置合并工具栏的单行尺寸，不依据图片宽高换行
pub(crate) fn toolbar_size() -> (f32, f32) {
    let config = crate::config::preferences::current();
    let mut toolbar = Toolbar::default();
    toolbar.items = Profile::Fullscreen.items(&config.toolbar_layout).to_vec();
    toolbar.orientation = Orientation::Horizontal;
    toolbar.arrange((100_000.0, 100_000.0));
    toolbar.size
}

/// 【图片窗口】【最小尺寸】无参数，返回可完整显示工具栏并保留画布的窗口最小宽高
pub(crate) fn minimum() -> (u32, u32) {
    let toolbar = toolbar_size();
    (
        (toolbar.0 + INSET * 2.0).ceil() as u32,
        (TITLE_HEIGHT + toolbar.1 + 180.0).ceil() as u32,
    )
}

/// 【图片窗口】【内容范围】按 size 返回标题和工具栏下方的图片视口，极小窗口仍保留非空区域
pub(crate) fn viewport(size: (u32, u32)) -> Rect {
    let top = TITLE_HEIGHT + toolbar_size().1 + INSET * 2.0;
    Rect::from_xywh(
        INSET,
        top,
        (size.0 as f32 - INSET * 2.0).max(1.0),
        (size.1 as f32 - top - INSET).max(1.0),
    )
    .unwrap()
}
