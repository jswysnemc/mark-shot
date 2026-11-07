//! 【截图工具栏】【图标映射】使用原版 SVG 路径，不另造工具图标

use super::{Integration, ToolbarButton};
use crate::ui::icons;

pub const CLEAR: &str = include_str!("../../../assets/icons/mark-shot/clear.svg");
pub const UNDO: &str = include_str!("../../../assets/icons/mark-shot/undo.svg");
pub const REDO: &str = include_str!("../../../assets/icons/mark-shot/redo.svg");
pub const SCOPE: &str = include_str!("../../../assets/icons/mark-shot/capture-scope.svg");
pub const LAYOUT: &str = include_str!("../../../assets/icons/mark-shot/toolbar-layout.svg");
pub const OPEN: &str = include_str!("../../../assets/icons/mark-shot/open-with.svg");
pub const EXTENSIONS: &str = include_str!("../../../assets/icons/mark-shot/extensions.svg");
pub const TRANSLATE: &str = include_str!("../../../assets/icons/mark-shot/translate.svg");
pub const UPLOAD: &str = include_str!("../../../assets/icons/mark-shot/upload.svg");
pub const SETTINGS: &str = include_str!("../../../assets/icons/mark-shot/settings.svg");
pub const EXTRA: &[&str] = &[
    CLEAR, UNDO, REDO, SCOPE, LAYOUT, OPEN, EXTENSIONS, TRANSLATE, UPLOAD, SETTINGS,
];

/// 【截图工具栏】【图标查找】返回 button 的内置 SVG；工具大小由主副工具栏统一控制
pub fn icon(button: ToolbarButton) -> &'static str {
    match button {
        ToolbarButton::Tool(tool) => icons::get_svg_for_tool(tool).0,
        ToolbarButton::Finish(finish) => icons::get_svg_for_finish(finish).0,
        ToolbarButton::SaveAs => icons::SAVE,
        ToolbarButton::Scroll => icons::SCROLL,
        ToolbarButton::Undo => UNDO,
        ToolbarButton::Redo => REDO,
        ToolbarButton::Clear => CLEAR,
        ToolbarButton::Scope => SCOPE,
        ToolbarButton::Layout => LAYOUT,
        ToolbarButton::Cancel => icons::CLOSE,
        ToolbarButton::Integration(integration) => match integration {
            Integration::OpenWith => OPEN,
            Integration::Extensions => EXTENSIONS,
            Integration::Translate => TRANSLATE,
            Integration::Upload => UPLOAD,
            Integration::Settings => SETTINGS,
        },
    }
}
