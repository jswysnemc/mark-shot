//! 【选区尺寸】【界面入口】紧凑字段、比例列表与自绘配色分别管理，不依赖浏览器或外部界面框架

pub mod cache;
mod draw;
pub mod field;
pub(crate) mod field_draw;
mod glyphs;
pub mod layout;
mod metrics;
pub mod navigation;
pub mod palette;
pub mod ratio_popup;
mod scroll;
pub mod state;
