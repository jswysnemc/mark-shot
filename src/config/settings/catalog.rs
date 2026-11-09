//! 【设置窗口】【分类目录】按使用任务组织设置，字段名与运行时配置保持一致

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Appearance,
    Capture,
    Keys,
    Ocr,
    Translation,
    Advanced,
    Toolbar,
    About,
    Storage,
    Pinned,
    Integrations,
}

impl Page {
    pub const ALL: [Self; 10] = [
        Self::Appearance,
        Self::Capture,
        Self::Toolbar,
        Self::Keys,
        Self::Ocr,
        Self::Translation,
        Self::Storage,
        Self::Pinned,
        Self::Integrations,
        Self::Advanced,
    ];
    /// 【设置窗口】【分类标题】按 chinese 返回当前分类名称，无副作用
    pub fn title(self, chinese: bool) -> &'static str {
        let labels = match self {
            Self::Appearance => ("外观与语言", "Appearance"),
            Self::Capture => ("截图与标注", "Capture"),
            Self::Keys => ("快捷键", "Shortcuts"),
            Self::Ocr => ("文字识别", "OCR"),
            Self::Translation => ("翻译服务", "Translation"),
            Self::Advanced => ("高级", "Advanced"),
            Self::Toolbar => ("工具栏", "Toolbar"),
            Self::Storage => ("存储与历史", "Storage"),
            Self::Pinned => ("贴图与外框", "Pins & frames"),
            Self::Integrations => ("上传与集成", "Integrations"),
            Self::About => ("关于", "About"),
        };
        if chinese { labels.0 } else { labels.1 }
    }
}

#[derive(Clone, Copy)]
pub enum Kind {
    Link(Page),
    Toggle,
    Choice(&'static [&'static str]),
    Segments(&'static [&'static str]),
    Text,
    Color,
    Shortcut,
}

#[derive(Clone)]
pub struct Entry {
    pub page: Page,
    pub section: &'static str,
    pub key: &'static str,
    pub labels: (&'static str, &'static str),
    pub kind: Kind,
}

impl Entry {
    /// 【设置窗口】【业务说明】按 chinese 返回设置作用和生效阶段，说明不显示内部配置键
    pub fn help(&self, chinese: bool) -> &'static str {
        let labels = match (self.section, self.key) {
            ("desktop", "tray_enabled") => (
                "后台保留截图、贴图、设置和退出菜单；需要桌面提供托盘区域",
                "Keeps capture, pin, settings and quit actions available in the desktop tray",
            ),
            ("desktop", "shortcuts_enabled") => (
                "在其他应用中也能截图；Wayland 由系统授权并确认绑定",
                "Capture from other apps; Wayland requires system shortcut approval",
            ),
            ("desktop", "screenshot_shortcut") => (
                "按下组合键重新绑定；系统占用或授权失败会显示原因",
                "Press a key combination to rebind; conflicts and permission errors are reported",
            ),
            ("ui", "language") => (
                "界面语言；环境变量覆盖时以环境为准",
                "Interface language; environment override takes priority",
            ),
            ("ui", "theme") => (
                "立即切换截图与原生窗口配色",
                "Immediately changes editor and window colors",
            ),
            ("notifications", "enabled") => (
                "复制或保存完成时发送桌面通知",
                "Desktop notification after copying or saving",
            ),
            ("notifications", "preview") => (
                "完成通知附带截图预览，不改变导出图片",
                "Screenshot thumbnail in completion notifications",
            ),
            ("general", "to") => (
                "按 Enter 或双击选区时执行此操作",
                "Action on Enter or double-clicking the selection",
            ),
            ("save", "directory") => (
                "绝对路径，或图片目录下的相对路径",
                "Absolute path, or relative to your Pictures folder",
            ),
            ("save", "filename_format") => (
                "使用日期格式，如 %Y-%m-%d_%H-%M-%S",
                "Date pattern, e.g. %Y-%m-%d_%H-%M-%S",
            ),
            ("save", "month_format") => (
                "月份子目录；留空直接存入保存目录",
                "Month subfolder; leave empty to save directly",
            ),
            ("startup", "include_cursor") => (
                "下次捕获时把系统指针写入截图",
                "Embeds the system pointer in the next capture",
            ),
            ("startup", "selection_loupe") => (
                "选择或调整选区时显示像素放大预览",
                "Pixel magnifier while selecting or resizing a region",
            ),
            ("startup", "normal_tool") => (
                "完成框选后自动进入所选标注工具",
                "Annotation tool selected after drawing a region",
            ),
            ("startup", "file_default_tool") => (
                "下次打开图片或贴图编辑时使用",
                "Initial tool when opening an image or editing a pin",
            ),
            ("region_detection", _) => (
                "下次截图悬停时识别图像内容边界",
                "Detects image regions when hovering in the next capture",
            ),
            ("window_detection", _) => (
                "下次截图悬停时识别窗口边界",
                "Detects window bounds when hovering in the next capture",
            ),
            ("history", _) => (
                "将完成的截图加入历史，可从历史重新打开",
                "Keeps completed captures available in history",
            ),
            ("keys", _) => (
                "截图编辑器内生效，不是桌面全局快捷键",
                "Editor shortcut, not a desktop-global hotkey",
            ),
            ("ocr", "mode") => (
                "下次截图生效；按需模式在识别后释放引擎",
                "Next capture; on-demand releases the engine after OCR",
            ),
            ("ocr", "engine") => (
                "下次截图生效；本地模型和推理库均需手动下载",
                "Next capture; local models and runtime require manual downloads",
            ),
            ("ocr", "resources") => (
                "查看安装状态、下载推理库与其他语言",
                "Inspect installation, download runtime and language models",
            ),
            ("startup", "default_color") => (
                "#RRGGBB 或 #RRGGBBAA；留空继承工具颜色",
                "#RRGGBB or #RRGGBBAA; empty inherits tool color",
            ),
            ("startup", "default_tool" | "fullscreen_default_tool") => (
                "留空沿用上级默认工具",
                "Empty inherits the common default tool",
            ),
            ("upload", "command") => (
                "主动上传时执行；环境变量可覆盖",
                "Runs on upload; environment may override",
            ),
            ("upload", "env") => (
                "键值均为字符串，如 {\"TOKEN\":\"value\"}",
                "String values, e.g. {\"TOKEN\":\"value\"}",
            ),
            ("ocr", "device") => (
                "下一次引擎初始化时使用；GPU 需要对应构建",
                "Used at engine initialization; GPU requires a GPU build",
            ),
            ("general", "overlay") => (
                "下次截图生效；自动按合成器能力选择",
                "Next capture; auto chooses based on compositor support",
            ),
            ("log", _) => (
                "下次启动生效，用于定位捕获或绘制问题",
                "Next launch; diagnostic output for capture and rendering",
            ),
            _ => ("", ""),
        };
        if chinese { labels.0 } else { labels.1 }
    }
    /// 【设置窗口】【字段名称】按 chinese 返回字段名称，配置键不作为主要界面文案
    pub fn label(&self, chinese: bool) -> &'static str {
        if chinese {
            self.labels.0
        } else {
            self.labels.1
        }
    }
}

macro_rules! entry {
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

pub(super) const BASE: &[Entry] = &[
    entry!(
        Appearance,
        "ui",
        "language",
        "界面语言",
        "Language",
        Kind::Choice(&["system", "chinese", "english"])
    ),
    entry!(
        Appearance,
        "ui",
        "theme",
        "界面主题",
        "Theme",
        Kind::Segments(&["light", "dark", "system"])
    ),
    entry!(
        Appearance,
        "notifications",
        "enabled",
        "完成后显示通知",
        "Completion notifications",
        Kind::Toggle
    ),
    entry!(
        Appearance,
        "notifications",
        "preview",
        "通知中显示缩略图",
        "Notification thumbnails",
        Kind::Toggle
    ),
    entry!(
        Capture,
        "general",
        "to",
        "默认完成操作",
        "Default output",
        Kind::Choice(&["c", "s", "cs", "p", "cp", "sp", "csp", ""])
    ),
    entry!(
        Storage,
        "save",
        "directory",
        "截图保存目录",
        "Screenshot directory",
        Kind::Text
    ),
    entry!(
        Storage,
        "save",
        "filename_format",
        "文件名格式",
        "Filename format",
        Kind::Text
    ),
    entry!(
        Capture,
        "startup",
        "include_cursor",
        "截图包含鼠标指针",
        "Include pointer",
        Kind::Toggle
    ),
    entry!(
        Capture,
        "startup",
        "selection_loupe",
        "选区放大预览",
        "Selection magnifier",
        Kind::Toggle
    ),
    entry!(
        Capture,
        "startup",
        "normal_tool",
        "选区完成后的工具",
        "Tool after selection",
        Kind::Choice(super::extra::TOOLS)
    ),
    entry!(
        Capture,
        "startup",
        "file_default_tool",
        "图片编辑默认工具",
        "Image editor tool",
        Kind::Choice(super::extra::TOOLS)
    ),
    entry!(
        Capture,
        "region_detection",
        "enabled",
        "自动识别内容区域",
        "Detect content regions",
        Kind::Toggle
    ),
    entry!(
        Capture,
        "window_detection",
        "enabled",
        "自动识别窗口",
        "Detect windows",
        Kind::Toggle
    ),
    entry!(
        Storage,
        "history",
        "enabled",
        "记录截图历史",
        "Capture history",
        Kind::Toggle
    ),
    entry!(
        Storage,
        "save",
        "month_format",
        "月份子目录格式",
        "Month subfolder pattern",
        Kind::Text
    ),
    entry!(
        Keys,
        "keys",
        "copy",
        "复制截图",
        "Copy screenshot",
        Kind::Shortcut
    ),
    entry!(
        Keys,
        "keys",
        "save",
        "保存截图",
        "Save screenshot",
        Kind::Shortcut
    ),
    entry!(
        Keys,
        "keys",
        "pin",
        "贴图",
        "Pin screenshot",
        Kind::Shortcut
    ),
    entry!(
        Keys,
        "keys",
        "scroll_capture",
        "滚动截图",
        "Scrolling capture",
        Kind::Shortcut
    ),
    entry!(
        Keys,
        "keys",
        "tool_eyedropper",
        "取色标注",
        "Color annotation",
        Kind::Shortcut
    ),
    entry!(
        Keys,
        "keys",
        "tool_text",
        "文字工具",
        "Text tool",
        Kind::Shortcut
    ),
    entry!(Keys, "keys", "undo", "撤销", "Undo", Kind::Shortcut),
    entry!(Keys, "keys", "redo", "重做", "Redo", Kind::Shortcut),
    entry!(
        Ocr,
        "ocr",
        "engine",
        "识别引擎",
        "OCR engine",
        Kind::Choice(if cfg!(target_os = "windows") {
            &["ask", "system", "paddle"]
        } else {
            &["paddle"]
        })
    ),
    entry!(
        Ocr,
        "ocr",
        "mode",
        "识别引擎运行方式",
        "Engine lifetime",
        Kind::Choice(&["on-demand", "at-launch", "daemon"])
    ),
    entry!(
        Ocr,
        "ocr",
        "device",
        "计算设备",
        "Compute device",
        Kind::Choice(if cfg!(feature = "ocr-gpu") {
            &["cpu", "auto", "gpu"]
        } else {
            &["cpu", "auto"]
        })
    ),
    entry!(
        Advanced,
        "general",
        "overlay",
        "截图窗口方式",
        "Capture surface",
        Kind::Choice(&["auto", "layer", "window"])
    ),
    entry!(
        Advanced,
        "log",
        "level",
        "日志级别",
        "Log level",
        Kind::Choice(&["off", "error", "info"])
    ),
];

/// 【设置窗口】【选项文案】将稳定配置 value 转为易读文案，chinese 控制界面语言，未知值原样显示
pub fn value_label(value: &str, chinese: bool) -> &str {
    if !chinese {
        return value;
    }
    match value {
        "system" => "跟随系统",
        "chinese" => "简体中文",
        "english" => "English",
        "light" => "浅色",
        "dark" => "深色",
        "c" => "复制",
        "s" => "保存",
        "cs" => "复制并保存",
        "p" => "贴图",
        "cp" => "复制并贴图",
        "sp" => "保存并贴图",
        "csp" => "复制、保存并贴图",
        "at-launch" => "截图时启动",
        "" => "继承默认",
        "highlighter" => "荧光笔",
        "laser" => "激光笔",
        "marker" => "标记",
        "ruler" => "标尺",
        "magnifier" => "放大镜",
        "number" => "编号",
        "line" => "直线",
        "circle" => "椭圆",
        "numerated-arrow" => "编号箭头",
        "redact" => "马赛克",
        "ocr" => "文字识别",
        "gpu" => "图形处理器",
        "true" => "已开启",
        "false" => "已关闭",
        "auto" => "自动",
        "cpu" => "处理器",
        "cuda" => "CUDA",
        "on-demand" => "按需启动",
        "ask" => "首次使用时选择",
        "paddle" => "本地模型 OCR",
        "daemon" => "常驻后台",
        "selection" => "选区",
        "pick" => "选择标注",
        "text" => "文字",
        "pen" => "画笔",
        "rectangle" => "矩形",
        "arrow" => "箭头",
        "eyedropper" => "取色标注",
        "layer" => "桌面覆盖层",
        "window" => "普通窗口",
        "off" => "关闭",
        "error" => "错误",
        "info" => "运行信息",
        _ => value,
    }
}
