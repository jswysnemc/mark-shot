//! 【设置窗口】【信息分组】定义页面说明、业务分组和导航图标

use super::catalog::{Entry, Page};
use crate::ui::icons;

/// 【设置窗口】【页面说明】按 page 和 chinese 返回页面作用，说明仅描述实际功能
pub(super) fn description(page: Page, chinese: bool) -> &'static str {
    let text = match page {
        Page::Appearance => (
            "调整界面语言、明暗配色与通知反馈",
            "Language, appearance and notification feedback",
        ),
        Page::Capture => (
            "配置截图行为、标注工具与文件保存方式",
            "Capture behavior, annotation tools and saved files",
        ),
        Page::Recording => (
            "录制质量、帧率、音频与保存位置",
            "Recording quality, frame rate, audio and destination",
        ),
        Page::Keys => (
            "点击快捷键录入；重复组合会提示冲突",
            "Click a shortcut to record; conflicting bindings are checked",
        ),
        Page::Ocr => (
            "选择识别引擎与运行方式；识别资源按需下载",
            "Choose an OCR engine; resources are installed on demand",
        ),
        Page::Translation => (
            "选择翻译服务，分别保存接口、模型与请求选项",
            "Choose a service and configure its connection, model and options",
        ),
        Page::Toolbar => (
            "主工具栏与侧工具栏分别设置，全屏时自动合并",
            "Configure the main and side toolbars; fullscreen combines both",
        ),
        Page::Advanced => (
            "管理窗口兼容性与诊断选项",
            "Window compatibility and diagnostic options",
        ),
        Page::About => (
            "轻量截图、标注与文字处理",
            "Lightweight capture, annotation and text tools",
        ),
        Page::Storage => (
            "保存路径、文件命名与历史保留策略",
            "Save paths, filenames and history retention",
        ),
        Page::Pinned => (
            "贴图边框与图片导出外框",
            "Pin borders and exported image frames",
        ),
        Page::Integrations => (
            "上传命令、环境变量与超时",
            "Upload command, environment and timeout",
        ),
    };
    if chinese { text.0 } else { text.1 }
}

/// 【设置窗口】【业务分组】按 entry 返回排序编号与双语分组标题，同组设置保持原有顺序
pub(super) fn group(entry: &Entry) -> (u8, &'static str, &'static str) {
    match entry.page {
        Page::Keys if entry.section == "desktop" => (0, "全局截图", "Global screenshot"),
        Page::Appearance if entry.section == "desktop" => (2, "系统托盘", "System tray"),
        Page::Storage if entry.section == "history" => (1, "截图历史", "History"),
        Page::Storage => (0, "文件保存", "Saved files"),
        Page::Pinned if entry.section == "pinned" => (0, "贴图", "Pins"),
        Page::Pinned => (1, "导出外框", "Export frame"),
        Page::Appearance if entry.section == "ui" => (0, "界面显示", "Interface"),
        Page::Appearance => (1, "系统通知", "Notifications"),
        Page::Capture => match entry.section {
            "general" => (0, "完成截图", "Capture actions"),
            "startup" | "magnifier" => (1, "截图与标注", "Capture and annotation"),
            "region_detection" | "window_detection" => (2, "自动识别", "Detection"),
            "history" => (3, "截图历史", "History"),
            _ => (4, "文件保存", "Saved files"),
        },
        Page::Keys if entry.key.starts_with("tool_") || matches!(entry.key, "undo" | "redo") => {
            (2, "标注与编辑", "Annotation and editing")
        }
        Page::Keys => (1, "编辑器快捷键", "Editor shortcuts"),
        Page::Ocr => (0, "识别引擎", "Recognition engine"),
        Page::Advanced if entry.section == "log" => (1, "诊断日志", "Diagnostics"),
        Page::Advanced if entry.section == "about" => (2, "应用信息", "Application"),
        Page::Advanced => (0, "窗口兼容性", "Window compatibility"),
        _ => (0, "设置", "Settings"),
    }
}

pub(super) const SUN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5"/></svg>"#;
pub(super) const KEYS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><rect x="3" y="5" width="18" height="14" rx="2"/><path d="M7 9h1m3 0h1m3 0h2M7 12h1m3 0h1m3 0h2M8 15h8"/></svg>"#;
pub(super) const PANEL: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18M9 9v12"/></svg>"#;
pub(super) const INFO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><circle cx="12" cy="12" r="9"/><path d="M12 11v6m0-10v.2"/></svg>"#;
pub(super) const SLIDERS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M4 6h4m4 0h8M4 12h10m4 0h2M4 18h2m4 0h10"/><circle cx="10" cy="6" r="2"/><circle cx="16" cy="12" r="2"/><circle cx="8" cy="18" r="2"/></svg>"#;

/// 【设置窗口】【导航图标】按 page 返回项目 SVG 或简洁线性图标，不使用字体符号
pub(super) fn icon(page: Page) -> &'static str {
    match page {
        Page::Appearance => SUN,
        Page::Capture => icons::RECTANGLE,
        Page::Recording => icons::VIDEO,
        Page::Keys => KEYS,
        Page::Ocr => icons::OCR,
        Page::Translation => icons::GLOBE,
        Page::Toolbar => PANEL,
        Page::Advanced => SLIDERS,
        Page::About => INFO,
        Page::Storage => icons::SAVE,
        Page::Pinned => icons::PIN,
        Page::Integrations => icons::GLOBE,
    }
}
