//! 【图片翻译】【共用模块】截图和贴图使用同一段落、配色、排版与坐标交换契约

pub mod colors;
pub mod exchange;
mod glyph_draw;
pub mod layout;
pub mod model;
pub mod paint;
pub mod paragraphs;
pub mod rect;
pub mod seed;
pub mod selection;
pub(crate) mod shaping;

pub use model::{SourceBlock, Token, TranslatedBlock, TranslatedLine};
pub use rect::BoxRect;
