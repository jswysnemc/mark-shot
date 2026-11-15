//! 【图片窗口】【布局边界】窗口优先容纳完整工具栏，原图适配其下方的可用画布

use crate::{
    config::toolbar_layout::Profile,
    ui::{
        image_window::{INSET, layout as bar},
        toolbar::{Orientation, Toolbar, ToolbarButton, ToolbarItem},
    },
};
use tiny_skia::Rect;

/// 【图片窗口】【编辑工具列表】按 config 返回图片编辑按钮，去除录屏及重复关闭入口，并整理剩余分隔线
pub(crate) fn toolbar_items(
    config: &crate::config::toolbar_layout::ToolbarLayout,
) -> Vec<ToolbarItem> {
    let mut items = Vec::new();
    let mut separator = false;
    for item in Profile::Fullscreen.items(config).iter().copied() {
        match item {
            ToolbarItem::Button(
                ToolbarButton::Cancel | ToolbarButton::Tool(crate::tools::Tool::Recording),
            ) => continue,
            ToolbarItem::Seperator => separator = !items.is_empty(),
            _ => {
                if separator {
                    items.push(ToolbarItem::Seperator);
                }
                items.push(item);
                separator = false;
            }
        }
    }
    items
}

/// 【图片窗口】【工具栏测量】无参数，返回当前配置合并工具栏的单行尺寸，不依据图片宽高换行
pub(crate) fn toolbar_size() -> (f32, f32) {
    let config = crate::config::preferences::current();
    let mut toolbar = Toolbar::default();
    toolbar.items = toolbar_items(&config.toolbar_layout);
    toolbar.orientation = Orientation::Horizontal;
    toolbar.arrange((100_000.0, 100_000.0));
    toolbar.size
}

/// 【图片窗口】【最小尺寸】无参数，返回可完整显示工具栏并保留画布的窗口最小宽高
pub(crate) fn minimum() -> (u32, u32) {
    let toolbar = toolbar_size();
    (
        (toolbar.0 + bar::BAR_INSET + bar::controls_width() + bar::DRAG_GAP).ceil() as u32,
        (bar::bar_height(toolbar.1) + 180.0 + INSET).ceil() as u32,
    )
}

/// 【图片窗口】【内容范围】按 size 返回合并工具栏下方的图片视口，极小窗口仍保留非空区域
pub(crate) fn viewport(size: (u32, u32)) -> Rect {
    let top = bar::bar_height(toolbar_size().1);
    Rect::from_xywh(
        INSET,
        top,
        (size.0 as f32 - INSET * 2.0).max(1.0),
        (size.1 as f32 - top - INSET).max(1.0),
    )
    .unwrap()
}

/// 【图片窗口】【工具位置】无参数，返回顶部合并栏中的工具起点，窗口按钮使用右侧保留空间
pub(crate) fn toolbar_position() -> (f32, f32) {
    (bar::BAR_INSET, 0.0)
}
