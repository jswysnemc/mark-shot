//! 【选区尺寸】【应用衔接】尺寸变更、面板布局、输入与原生弹层分别处理，比例只属于当前截图会话

pub(in crate::app) mod apply;
pub(in crate::app) mod caret;
mod field_clicks;
pub(in crate::app) mod field_clipboard;
mod field_pointer;
pub(in crate::app) mod input;
pub(in crate::app) mod input_method;
pub(in crate::app) mod keyboard;
pub(in crate::app) mod panels;
pub(in crate::app) mod popup;

use crate::ui::selection_size::{ratio_popup::Popup, state::Panel};

#[derive(Default)]
pub(crate) struct State {
    pub panel: Option<Panel>,
    pub popup: Option<Popup>,
    pub caret: crate::features::text::caret::Caret,
    pub pressed: Option<crate::types::MouseButton>,
    pub field_drag: Option<field_pointer::Drag>,
    clipboard: field_clipboard::Clipboard,
    pub(in crate::app) clicks: field_clicks::Clicks,
    pub(in crate::app) input_layout: Option<input_method::Layout>,
}

impl State {
    /// 【选区尺寸】【输入交互】返回是否有活动字段或比例弹层，无参数，静止显示不占用输入
    pub fn interacting(&self) -> bool {
        self.popup.is_some()
            || self
                .panel
                .as_ref()
                .is_some_and(|panel| panel.editing().is_some())
    }
}
