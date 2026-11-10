//! 【截图配置】【行为模型】截图范围、窗口排除和鼠标动作的统一配置

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClickAction {
    #[default]
    #[serde(alias = "default", alias = "pan", alias = "zoom")]
    None,
    Copy,
    Save,
    #[serde(alias = "saveAs", alias = "save_as")]
    SaveAs,
    Pin,
    Cancel,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FreezeScope {
    #[default]
    AllScreens,
    CursorScreen,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolbarPosition {
    #[default]
    Selection,
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Capture {
    #[serde(alias = "freezeScope")]
    pub freeze_scope: FreezeScope,
    #[serde(alias = "hideOwnWindows", alias = "hideOwnWindowsDuringCapture")]
    pub hide_own_windows: bool,
    #[serde(alias = "doubleClickAction")]
    pub double_click_action: ClickAction,
    #[serde(alias = "middleClickAction")]
    pub middle_click_action: ClickAction,
    #[serde(alias = "toolbarPosition")]
    pub toolbar_position: ToolbarPosition,
    pub kde_kwin_screenshot: bool,
}

impl Default for Capture {
    /// 【截图配置】【默认值】无参数，返回 Qt 的全屏冻结、双击复制及中键仅平移策略
    fn default() -> Self {
        Self {
            freeze_scope: FreezeScope::AllScreens,
            hide_own_windows: true,
            double_click_action: ClickAction::Copy,
            middle_click_action: ClickAction::None,
            toolbar_position: ToolbarPosition::Selection,
            kde_kwin_screenshot: true,
        }
    }
}

impl Capture {
    /// 【截图配置】【范围优先级】按显式单屏、全部输出和全屏模式返回是否仅冻结光标屏幕
    pub fn one_monitor(&self, explicit_one: bool, explicit_all: bool, fullscreen: bool) -> bool {
        explicit_one
            || (!explicit_all && !fullscreen && self.freeze_scope == FreezeScope::CursorScreen)
    }
}
