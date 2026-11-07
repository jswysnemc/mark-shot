//! 【界面控件】【多行编辑】共享文字编辑、选择与增量撤销控件

mod draw;
#[cfg(test)]
mod draw_tests;
mod input;
mod input_method;
mod state;
mod style;

pub use input::EditEvent;
pub use input_method::composition::Update as InputMethodUpdate;
pub use input_method::context::InputArea;
pub use state::TextEditor;
pub use style::EditorStyle;
