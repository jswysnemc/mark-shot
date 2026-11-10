//! 【截图设置】【字段目录】截图策略与工具栏外观共用既有设置控件

use super::{Entry, Kind, Page};

const ACTIONS: &[&str] = &["none", "copy", "save", "save-as", "pin", "cancel"];

/// 【截图设置】【目录构造】无参数，返回可编辑字段；字段值同时用于原生配置和 Qt JSON
pub(super) fn entries() -> Vec<Entry> {
    [
        (
            Page::Capture,
            "capture",
            "double_click_action",
            "选区双击动作",
            "Selection double-click",
            Kind::Choice(ACTIONS),
        ),
        (
            Page::Capture,
            "capture",
            "middle_click_action",
            "选区中键单击动作",
            "Selection middle-click",
            Kind::Choice(ACTIONS),
        ),
        (
            Page::Capture,
            "capture",
            "freeze_scope",
            "显示器冻结范围",
            "Display freeze scope",
            Kind::Choice(&["all-screens", "cursor-screen"]),
        ),
        (
            Page::Capture,
            "capture",
            "hide_own_windows",
            "截图时隐藏自身窗口",
            "Hide own windows during capture",
            Kind::Toggle,
        ),
        (
            Page::Capture,
            "capture",
            "kde_kwin_screenshot",
            "使用 KDE KWin 截图",
            "Use KDE KWin screenshots",
            Kind::Toggle,
        ),
        (
            Page::Toolbar,
            "capture",
            "toolbar_position",
            "工具栏位置",
            "Toolbar position",
            Kind::Choice(&["selection", "top", "bottom", "left", "right"]),
        ),
        (
            Page::Toolbar,
            "toolbar",
            "icon_size",
            "图标尺寸（像素）",
            "Icon size (px)",
            Kind::Text,
        ),
        (
            Page::Toolbar,
            "toolbar",
            "font_size",
            "工具栏字号（像素）",
            "Toolbar font size (px)",
            Kind::Text,
        ),
    ]
    .into_iter()
    .map(|(page, section, key, cn, en, kind)| Entry {
        page,
        section,
        key,
        labels: (cn, en),
        kind,
    })
    .collect()
}

/// 【截图设置】【选项文案】按字段、值和语言返回局部文案，其他字段返回空值
pub(super) fn label(key: &str, value: &str, chinese: bool) -> Option<&'static str> {
    let labels = match (key, value) {
        ("double_click_action" | "middle_click_action", "none") => {
            ("不执行动作（保留拖动）", "No action (keep dragging)")
        }
        ("double_click_action" | "middle_click_action", "copy") => ("复制并关闭", "Copy and close"),
        ("double_click_action" | "middle_click_action", "save") => ("保存并关闭", "Save and close"),
        ("double_click_action" | "middle_click_action", "save-as") => ("另存为", "Save as"),
        ("double_click_action" | "middle_click_action", "pin") => ("贴图", "Pin image"),
        ("double_click_action" | "middle_click_action", "cancel") => ("放弃截图", "Cancel capture"),
        ("freeze_scope", "all-screens") => ("所有显示器", "All displays"),
        ("freeze_scope", "cursor-screen") => ("鼠标所在显示器", "Display under cursor"),
        ("toolbar_position", "selection") => ("跟随选区", "Follow selection"),
        ("toolbar_position", "top") => ("屏幕顶部", "Screen top"),
        ("toolbar_position", "bottom") => ("屏幕底部", "Screen bottom"),
        ("toolbar_position", "left") => ("屏幕左侧", "Screen left"),
        ("toolbar_position", "right") => ("屏幕右侧", "Screen right"),
        _ => return None,
    };
    Some(if chinese { labels.0 } else { labels.1 })
}

/// 【截图设置】【帮助文案】按字段和语言返回动作边界，未知字段返回空值
pub(super) fn help(key: &str, chinese: bool) -> Option<&'static str> {
    let labels = match key {
        "double_click_action" => (
            "双击选区空白处执行；文字编辑和标注手柄优先",
            "Applies to empty selection space; text and annotation handles take priority",
        ),
        "middle_click_action" => (
            "中键按下后不拖动才执行；拖动仍平移画布",
            "Runs on release without dragging; dragging still pans the canvas",
        ),
        "freeze_scope" => (
            "普通区域截图的默认范围；命令行指定显示器或全部输出时优先采用命令行",
            "Default for region capture; explicit display and all-output options take priority",
        ),
        "hide_own_windows" => (
            "捕获桌面前临时隐藏贴图等自身窗口，捕获结束后恢复",
            "Temporarily hides application windows before capture and restores them afterward",
        ),
        "kde_kwin_screenshot" => (
            "关闭后 KDE 截图使用桌面门户，可能出现系统授权窗口",
            "When off, KDE uses the desktop portal and may request permission",
        ),
        "toolbar_position" => (
            "固定停靠不随选区移动，空间不足时两条工具栏自动错行",
            "Docked toolbars stay at the screen edge and wrap when space is limited",
        ),
        "icon_size" => (
            "同步调整工具栏图标、按钮和点击区域",
            "Resizes toolbar icons, buttons and hit areas together",
        ),
        "font_size" => (
            "调整截图属性栏文字和数值控件字号",
            "Sets the font size of capture properties and numeric controls",
        ),
        _ => return None,
    };
    Some(if chinese { labels.0 } else { labels.1 })
}
