//! 【截图场景】【提交契约】后端负责背景与遮罩，前景仍由共享编辑器绘制

pub struct Frame<'a> {
    pub foreground: &'a mut tiny_skia::Pixmap,
    pub background: &'a crate::image_source::ImageSource,
    pub damage: Option<super::DamageRect>,
    pub selection: Option<tiny_skia::Rect>,
    pub hovered: Option<tiny_skia::Rect>,
    pub opaque: bool,
}
