//! 【图片窗口】【系统动作】自绘标题栏与各平台普通窗口共用的操作契约

use crate::backend::window::WindowEdge;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Move,
    Resize(WindowEdge),
    Minimize,
    ToggleMaximize,
}
