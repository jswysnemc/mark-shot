//! 【选择面板】【入口】共用紧凑网格、滚动、键盘选择和透明度动画

mod draw;
mod layout;
mod state;

pub use draw::draw_choice_popover;
pub use layout::ChoiceLayout;
pub use state::{ChoiceEvent, ChoiceOption, ChoicePopover};
