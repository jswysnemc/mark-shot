//! 【贴图】【菜单定义】原版菜单顺序与任务可用状态，翻译入口主动检查配置并报告实际错误

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinAction {
    RotateLeft,
    RotateRight,
    ZoomIn,
    ZoomOut,
    ResetSize,
    AlwaysOnTop,
    Edit,
    Copy,
    CopySelectedText,
    CopyImageText,
    ScanCode,
    TextSelection,
    Translate,
    OpenTranslationWindow,
    TargetLanguage,
    ToggleTranslation,
    SaveAs,
    Close,
}

#[derive(Debug, Clone, Copy)]
pub struct MenuItem {
    pub label: &'static str,
    pub action: Option<PinAction>,
}

use PinAction as A;
pub const ITEMS: &[MenuItem] = &[
    MenuItem {
        label: "Rotate Left",
        action: Some(A::RotateLeft),
    },
    MenuItem {
        label: "Rotate Right",
        action: Some(A::RotateRight),
    },
    MenuItem {
        label: "",
        action: None,
    },
    MenuItem {
        label: "Zoom In",
        action: Some(A::ZoomIn),
    },
    MenuItem {
        label: "Zoom Out",
        action: Some(A::ZoomOut),
    },
    MenuItem {
        label: "Reset Size",
        action: Some(A::ResetSize),
    },
    MenuItem {
        label: "",
        action: None,
    },
    MenuItem {
        label: "Always on Top",
        action: Some(A::AlwaysOnTop),
    },
    MenuItem {
        label: "",
        action: None,
    },
    MenuItem {
        label: "Edit",
        action: Some(A::Edit),
    },
    MenuItem {
        label: "Copy",
        action: Some(A::Copy),
    },
    MenuItem {
        label: "Copy Selected Text",
        action: Some(A::CopySelectedText),
    },
    MenuItem {
        label: "Copy Image Text",
        action: Some(A::CopyImageText),
    },
    MenuItem {
        label: "Scan QR / barcode",
        action: Some(A::ScanCode),
    },
    MenuItem {
        label: "Text Selection Copy",
        action: Some(A::TextSelection),
    },
    MenuItem {
        label: "Translate",
        action: Some(A::Translate),
    },
    MenuItem {
        label: "Open Translation Window",
        action: Some(A::OpenTranslationWindow),
    },
    MenuItem {
        label: "Target Language",
        action: Some(A::TargetLanguage),
    },
    MenuItem {
        label: "Show Translated Text",
        action: Some(A::ToggleTranslation),
    },
    MenuItem {
        label: "Save As",
        action: Some(A::SaveAs),
    },
    MenuItem {
        label: "",
        action: None,
    },
    MenuItem {
        label: "Close",
        action: Some(A::Close),
    },
];

pub const WIDTH: u32 = 232;
pub const PADDING: u32 = 4;
pub const ROW: u32 = 28;
pub const SEPARATOR: u32 = 9;

/// 【贴图】【菜单高度】返回全部菜单行及边距的逻辑高度
pub fn height() -> u32 {
    PADDING * 2
        + ITEMS
            .iter()
            .map(|item| {
                if item.action.is_some() {
                    ROW
                } else {
                    SEPARATOR
                }
            })
            .sum::<u32>()
}

/// 【贴图】【菜单命中】按 position 查找菜单行；空白与分隔线返回 None
pub fn hit(position: (f64, f64)) -> Option<usize> {
    if position.0 < f64::from(PADDING) || position.0 >= f64::from(WIDTH - PADDING) {
        return None;
    }
    let mut top = f64::from(PADDING);
    for (index, item) in ITEMS.iter().enumerate() {
        let bottom = top
            + f64::from(if item.action.is_some() {
                ROW
            } else {
                SEPARATOR
            });
        if position.1 >= top && position.1 < bottom {
            return item.action.map(|_| index);
        }
        top = bottom;
    }
    None
}

/// 【贴图】【菜单可用性】返回 action 能否执行；文字选择依赖已就绪结果，置顶依赖协议支持
pub fn enabled(
    action: PinAction,
    top_supported: bool,
    has_selection: bool,
    _has_text: bool,
    translation_busy: bool,
    has_translation: bool,
) -> bool {
    match action {
        A::AlwaysOnTop => top_supported,
        A::CopySelectedText => has_selection,
        A::TextSelection => true,
        A::Translate => !translation_busy,
        A::TargetLanguage => true,
        A::ToggleTranslation => has_translation && !translation_busy,
        _ => true,
    }
}

/// 【贴图】【菜单行顶边】按 index 返回菜单行的逻辑顶边，非法索引返回空值，用于原生语言子菜单定位
pub fn row_top(index: usize) -> Option<u32> {
    ITEMS.get(index)?;
    Some(
        PADDING
            + ITEMS[..index]
                .iter()
                .map(|item| {
                    if item.action.is_some() {
                        ROW
                    } else {
                        SEPARATOR
                    }
                })
                .sum::<u32>(),
    )
}
