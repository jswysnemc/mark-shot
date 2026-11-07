//! 【文字识别】【控件文字】集中管理原版两种界面语言及固定字号下的控件尺寸

use super::window::View;
use crate::ui::language::Language;

pub(super) const TITLE_FONT: f32 = 12.0;
pub(super) const BODY_FONT: f32 = 15.0;
pub(super) const BODY_LINE_HEIGHT: f32 = 20.0;

/// 【文字识别】【页面文字】按 view 返回原版页签名称，无其他参数
pub(super) fn view(view: View) -> &'static str {
    match view {
        View::Text => "Text",
        View::Table => "Table",
        View::Image => "Source image",
    }
}

/// 【文字识别】【页签尺寸】按 view 返回原版 Noto Sans 12 像素页签宽度，包含内边距及最小省略宽度
pub(super) fn tab_width(view: View) -> f32 {
    match view {
        View::Text => 43.0,
        View::Table => 48.0,
        View::Image => 95.0,
    }
}

/// 【文字识别】【本地化页签】按 view、language 返回原版页签宽度，中文各页均使用实测的 42 像素
pub(super) fn localized_tab_width(view: View, language: Language) -> f32 {
    if language == Language::Chinese {
        42.0
    } else {
        tab_width(view)
    }
}

/// 【文字识别】【复制尺寸】按 label 返回原版图标、间距、文字及内边距的合计宽度，未知反馈不扩大控件
pub(super) fn copy_width(label: &str) -> f32 {
    match label {
        "复制" => 62.0,
        "已复制" => 74.0,
        "复制失败" => 87.0,
        "Copied" => 78.0,
        "Copy failed" => 101.0,
        _ => 68.0,
    }
}

/// 【文字识别】【表格复制尺寸】按 label、language 返回原版表格复制宽度，常态保持 68 像素，失败分别使用英文 90、中文 75 像素
pub(super) fn table_copy_width(label: &str, language: Language) -> f32 {
    if matches!(label, "Copy failed" | "复制失败") {
        if language == Language::Chinese {
            75.0
        } else {
            90.0
        }
    } else {
        68.0
    }
}
