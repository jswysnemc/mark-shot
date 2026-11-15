//! 【图片窗口】【窗口状态】工具栏兼任顶部窗口栏，窗口操作独立于图片缩放和标注导出

mod controls;
mod draw;
pub(crate) mod layout;

pub(crate) use draw::draw;
pub(crate) use layout::INSET;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hit {
    Move,
    Minimize,
    Maximize,
    Close,
    Resize(crate::backend::window::WindowEdge),
}

#[derive(Default)]
pub(crate) struct State {
    pub enabled: bool,
    pub hovered: Option<Hit>,
    pub pressed: Option<Hit>,
    pub action: Option<crate::types::image_window::Action>,
}
