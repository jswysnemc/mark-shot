//! 【截图工具栏】【按钮定义】保留原版分组顺序，工具状态与业务动作分别映射

use crate::{keys::Action, tools::Tool, types::Finish, ui::panel::PanelItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Integration {
    OpenWith,
    Extensions,
    Translate,
    Upload,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarButton {
    Tool(Tool),
    Finish(Finish),
    SaveAs,
    Scroll,
    Undo,
    Redo,
    Clear,
    Scope,
    Layout,
    Cancel,
    Integration(Integration),
}

impl ToolbarButton {
    /// 【截图工具栏】【动作映射】返回按钮对应的通用动作；工具栏专有动作返回 None
    pub fn action(self) -> Option<Action> {
        match self {
            Self::Tool(tool) => Some(Action::Tool(tool)),
            Self::Finish(finish) => Some(Action::Finish(finish)),
            Self::SaveAs => Some(Action::SaveAs),
            Self::Integration(Integration::OpenWith) => Some(Action::OpenWith),
            Self::Integration(Integration::Extensions) => Some(Action::Extensions),
            Self::Integration(Integration::Upload) => Some(Action::Upload),
            Self::Integration(Integration::Translate) => Some(Action::Translate),
            Self::Scroll => Some(Action::ScrollCapture),
            Self::Undo => Some(Action::Undo),
            Self::Redo => Some(Action::Redo),
            Self::Cancel => Some(Action::Cancel),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarItem {
    Button(ToolbarButton),
    Seperator,
}

impl PanelItem for ToolbarItem {
    /// 【截图工具栏】【项目宽度】返回主工具栏逻辑像素宽度；分隔线不增加额外装饰留白
    fn size(&self) -> f32 {
        match self {
            Self::Button(_) => crate::config::get().toolbar.button_size,
            Self::Seperator => 1.0,
        }
    }

    /// 【截图工具栏】【项目间距】返回原版按钮间距，末项间距由布局统一扣除
    fn trailing_padding(&self) -> f32 {
        crate::config::get().toolbar.button_gap
    }

    /// 【截图工具栏】【项目分类】仅业务按钮返回 true
    fn is_button(&self) -> bool {
        matches!(self, Self::Button(_))
    }
}

use ToolbarButton as B;
use ToolbarItem::{Button as I, Seperator as S};

pub const ITEMS: &[ToolbarItem] = &[
    I(B::Tool(Tool::Selection)),
    I(B::Tool(Tool::Pick)),
    S,
    I(B::Tool(Tool::Pen)),
    I(B::Tool(Tool::Line)),
    I(B::Tool(Tool::Highlighter)),
    S,
    I(B::Tool(Tool::Rectangle)),
    I(B::Tool(Tool::Circle)),
    I(B::Tool(Tool::Marker)),
    I(B::Tool(Tool::Arrow)),
    S,
    I(B::Tool(Tool::Text)),
    I(B::Tool(Tool::Number)),
    I(B::Tool(Tool::Redact)),
    I(B::Tool(Tool::Magnifier)),
    I(B::Tool(Tool::Ruler)),
    I(B::Tool(Tool::Laser)),
    S,
    I(B::Clear),
    I(B::Undo),
    I(B::Redo),
];

pub const ACTION_ITEMS: &[ToolbarItem] = &[
    I(B::Scope),
    S,
    I(B::Integration(Integration::OpenWith)),
    I(B::Integration(Integration::Extensions)),
    I(B::Scroll),
    I(B::Finish(Finish::Pin)),
    I(B::Tool(Tool::Ocr)),
    I(B::Integration(Integration::Translate)),
    S,
    I(B::Finish(Finish::Copy)),
    I(B::SaveAs),
    I(B::Integration(Integration::Upload)),
    S,
    I(B::Integration(Integration::Settings)),
    S,
    I(B::Cancel),
];

// 全图编辑将副栏动作合并到主栏，保持原版只显示一条工具栏
pub const FULLSCREEN_ITEMS: &[ToolbarItem] = &[
    I(B::Tool(Tool::Selection)),
    I(B::Tool(Tool::Pick)),
    S,
    I(B::Tool(Tool::Pen)),
    I(B::Tool(Tool::Line)),
    I(B::Tool(Tool::Highlighter)),
    S,
    I(B::Tool(Tool::Rectangle)),
    I(B::Tool(Tool::Circle)),
    I(B::Tool(Tool::Marker)),
    I(B::Tool(Tool::Arrow)),
    S,
    I(B::Tool(Tool::Text)),
    I(B::Tool(Tool::Number)),
    I(B::Tool(Tool::Redact)),
    I(B::Tool(Tool::Magnifier)),
    I(B::Tool(Tool::Ruler)),
    I(B::Tool(Tool::Laser)),
    S,
    I(B::Clear),
    I(B::Undo),
    I(B::Redo),
    S,
    I(B::Scope),
    I(B::Layout),
    S,
    I(B::Integration(Integration::OpenWith)),
    I(B::Integration(Integration::Extensions)),
    I(B::Scroll),
    I(B::Finish(Finish::Pin)),
    I(B::Tool(Tool::Ocr)),
    I(B::Integration(Integration::Translate)),
    I(B::Finish(Finish::Copy)),
    I(B::SaveAs),
    I(B::Integration(Integration::Upload)),
    S,
    I(B::Integration(Integration::Settings)),
    S,
    I(B::Cancel),
];
