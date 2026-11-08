//! 【应用设置】【动作目录】从运行时动作表生成全部快捷键，避免遗漏新工具
use super::{Entry, Kind, Page};

/// 【应用设置】【快捷键目录】无参数，返回全部可配置动作，默认绑定始终读取动作表
pub(super) fn entries() -> Vec<Entry> {
    crate::keys::ACTIONS
        .iter()
        .map(|(key, _, _)| Entry {
            page: Page::Keys,
            section: "keys",
            key,
            labels: labels(key),
            kind: Kind::Shortcut,
        })
        .collect()
}

/// 【应用设置】【动作文案】按 key 返回中英文名称，未知动作保留键名供新动作直接显示
fn labels(key: &'static str) -> (&'static str, &'static str) {
    match key {
        "accept" => ("完成截图", "Finish capture"),
        "copy" => ("复制截图", "Copy"),
        "save" => ("保存截图", "Save"),
        "save_as" => ("另存为", "Save as"),
        "open_with" => ("打开方式", "Open with"),
        "extensions" => ("扩展操作", "Extensions"),
        "upload" => ("上传截图", "Upload"),
        "translate" => ("翻译", "Translate"),
        "pin" => ("贴图", "Pin"),
        "cancel" => ("取消截图", "Cancel"),
        "undo" => ("撤销", "Undo"),
        "redo" => ("重做", "Redo"),
        "select_all" => ("选择全部", "Select all"),
        "delete" => ("删除标注", "Delete"),
        "toggle_ui" => ("显示或隐藏工具栏", "Toggle toolbar"),
        "toggle_magnifier" => ("选区放大预览", "Selection magnifier"),
        "toggle_theme" => ("切换明暗主题", "Toggle theme"),
        "scroll_capture" => ("滚动截图", "Scrolling capture"),
        "history" => ("截图历史", "History"),
        "settings" => ("设置", "Settings"),
        "clear_annotations" => ("清空标注", "Clear annotations"),
        "toggle_scope" => ("切换截图范围", "Toggle capture scope"),
        "toggle_toolbar_layout" => ("切换工具栏排列", "Toggle toolbar layout"),
        "size_up" => ("增加工具大小", "Increase tool size"),
        "size_down" => ("减小工具大小", "Decrease tool size"),
        "tool_selection" => ("选区", "Selection"),
        "tool_pick" => ("选择标注", "Pick"),
        "tool_ocr" => ("文字识别", "OCR"),
        "tool_eyedropper" => ("取色标注", "Eyedropper"),
        "tool_text" => ("文字", "Text"),
        "tool_pen" => ("画笔", "Pen"),
        "tool_highlighter" => ("荧光笔", "Highlighter"),
        "tool_laser" => ("激光笔", "Laser"),
        "tool_marker" => ("标记", "Marker"),
        "tool_ruler" => ("标尺", "Ruler"),
        "tool_magnifier" => ("放大镜", "Magnifier"),
        "tool_number" => ("编号", "Number"),
        "tool_line" => ("直线", "Line"),
        "tool_arrow" => ("箭头", "Arrow"),
        "tool_rectangle" => ("矩形", "Rectangle"),
        "tool_circle" => ("椭圆", "Ellipse"),
        "tool_numerated_arrow" => ("编号箭头", "Numbered arrow"),
        "tool_redact" => ("马赛克", "Mosaic"),
        "move_left" => ("向左移动", "Move left"),
        "move_right" => ("向右移动", "Move right"),
        "move_up" => ("向上移动", "Move up"),
        "move_down" => ("向下移动", "Move down"),
        "move_left_fast" => ("快速向左移动", "Move left quickly"),
        "move_right_fast" => ("快速向右移动", "Move right quickly"),
        "move_up_fast" => ("快速向上移动", "Move up quickly"),
        "move_down_fast" => ("快速向下移动", "Move down quickly"),
        "resize_left" => ("向左调整选区", "Resize left"),
        "resize_right" => ("向右调整选区", "Resize right"),
        "resize_up" => ("向上调整选区", "Resize up"),
        "resize_down" => ("向下调整选区", "Resize down"),
        "resize_left_fast" => ("快速向左调整选区", "Resize left quickly"),
        "resize_right_fast" => ("快速向右调整选区", "Resize right quickly"),
        "resize_up_fast" => ("快速向上调整选区", "Resize up quickly"),
        "resize_down_fast" => ("快速向下调整选区", "Resize down quickly"),
        _ => (key, key),
    }
}
