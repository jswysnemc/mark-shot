//! 【应用设置】【业务目录】存储、贴图、外框、上传与启动偏好的可编辑定义
use super::{Entry, Kind, Page};
macro_rules! field {
    ($page:ident, $section:literal, $key:literal, $cn:literal, $en:literal, $kind:expr) => {
        Entry {
            page: Page::$page,
            section: $section,
            key: $key,
            labels: ($cn, $en),
            kind: $kind,
        }
    };
}
pub(super) const TOOLS: &[&str] = &[
    "",
    "selection",
    "pick",
    "text",
    "pen",
    "highlighter",
    "laser",
    "marker",
    "ruler",
    "magnifier",
    "number",
    "line",
    "arrow",
    "rectangle",
    "circle",
    "numerated-arrow",
    "redact",
    "eyedropper",
    "ocr",
];
pub(super) const ENTRIES: &[Entry] = &[
    field!(
        Capture,
        "magnifier",
        "zoom",
        "光标放大区倍率",
        "Cursor loupe scale",
        Kind::Text
    ),
    field!(
        Ocr,
        "ocr",
        "resources",
        "识别语言与本地资源",
        "Languages and local resources",
        Kind::Link(Page::Ocr)
    ),
    field!(
        Toolbar,
        "toolbar_layout",
        "layout",
        "主栏、侧栏与工具池",
        "Main toolbar, side toolbar and tool pool",
        Kind::Link(Page::Toolbar)
    ),
    field!(
        Capture,
        "startup",
        "default_tool",
        "默认标注工具",
        "Default tool",
        Kind::Choice(TOOLS)
    ),
    field!(
        Capture,
        "startup",
        "fullscreen_default_tool",
        "全屏截图默认工具",
        "Fullscreen tool",
        Kind::Choice(TOOLS)
    ),
    field!(
        Capture,
        "startup",
        "default_color",
        "默认标注颜色",
        "Default annotation color",
        Kind::Text
    ),
    field!(
        Storage,
        "history",
        "limit",
        "历史保留数量（1–200）",
        "History count (1–200)",
        Kind::Text
    ),
    field!(
        Storage,
        "history",
        "max_size_mib",
        "历史容量上限（MiB）",
        "History size limit (MiB)",
        Kind::Text
    ),
    field!(
        Storage,
        "history",
        "directory",
        "历史存储目录",
        "History directory",
        Kind::Text
    ),
    field!(
        Storage,
        "history",
        "index_annotations",
        "索引标注文字",
        "Index annotation text",
        Kind::Toggle
    ),
    field!(
        Pinned,
        "pinned",
        "always_on_top",
        "贴图始终置顶",
        "Keep pins on top",
        Kind::Toggle
    ),
    field!(
        Pinned,
        "pinned",
        "border_enabled",
        "显示贴图边框",
        "Pin border",
        Kind::Toggle
    ),
    field!(
        Pinned,
        "pinned",
        "border_color",
        "贴图边框颜色",
        "Pin border color",
        Kind::Text
    ),
    field!(
        Pinned,
        "pinned",
        "border_width",
        "贴图边框宽度",
        "Pin border width",
        Kind::Text
    ),
    field!(
        Pinned,
        "export.image_frame",
        "enabled",
        "导出时添加外框",
        "Export image frame",
        Kind::Toggle
    ),
    field!(
        Pinned,
        "export.image_frame",
        "padding",
        "外框留白",
        "Frame padding",
        Kind::Text
    ),
    field!(
        Pinned,
        "export.image_frame",
        "corner_radius",
        "外框圆角",
        "Frame corner radius",
        Kind::Text
    ),
    field!(
        Pinned,
        "export.image_frame",
        "shadow_radius",
        "阴影半径",
        "Shadow radius",
        Kind::Text
    ),
    field!(
        Pinned,
        "export.image_frame",
        "shadow_offset_y",
        "阴影垂直偏移",
        "Shadow vertical offset",
        Kind::Text
    ),
    field!(
        Pinned,
        "export.image_frame",
        "shadow_opacity",
        "阴影不透明度（0–1）",
        "Shadow opacity (0–1)",
        Kind::Text
    ),
    field!(
        Integrations,
        "upload",
        "command",
        "上传命令",
        "Upload command",
        Kind::Text
    ),
    field!(
        Integrations,
        "upload",
        "timeout_ms",
        "上传超时（毫秒）",
        "Upload timeout (ms)",
        Kind::Text
    ),
    field!(
        Integrations,
        "upload",
        "env",
        "上传环境变量（JSON）",
        "Upload environment (JSON)",
        Kind::Text
    ),
    field!(
        Ocr,
        "ocr",
        "result_panel",
        "显示文字识别结果面板",
        "OCR result panel",
        Kind::Toggle
    ),
    field!(
        Ocr,
        "ocr_result_window",
        "always_on_top",
        "识别结果窗口置顶",
        "OCR result window on top",
        Kind::Toggle
    ),
    field!(
        Advanced,
        "log",
        "path",
        "日志文件位置",
        "Log file path",
        Kind::Text
    ),
    field!(
        Advanced,
        "about",
        "version",
        "版本与应用信息",
        "Version and application information",
        Kind::Link(Page::About)
    ),
];
