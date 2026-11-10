//! 【原生窗口】【接口】普通 xdg-toplevel 窗口与内容之间的事件契约

mod capture_visibility;
mod deadline;
mod dialog;
mod dialog_protocol;
mod input;
mod input_grab;
mod layer_geometry;
mod popup;
mod popup_input;
mod popup_resources;
mod protocol;
mod role;
mod runtime;
mod scroll;
mod shell;
mod single_instance;
mod stacking;
mod surface;
mod text_input;
mod theme_switch;
mod wake;

pub use crate::backend::window::{
    KeyInput, LEFT_BUTTON, RIGHT_BUTTON, ScrollInput, ScrollSource, WindowAction, WindowContent,
    WindowCursor, WindowDialog, WindowEvent, WindowPopupAction, WindowSpec,
};
pub use runtime::run;
pub use wake::WindowWake;
