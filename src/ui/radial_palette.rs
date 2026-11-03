//! 【快捷色板】【组件入口】原版透明环形色板，不与属性取色器共用控件布局

mod draw;
mod state;
pub use draw::draw;
pub use state::{COLORS, RadialPalette, SIZE};
