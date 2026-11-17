//! 【截图场景】【提交契约】后端负责背景与遮罩，全部几何范围统一采用与前景画布一致的物理像素坐标

pub struct Frame<'a> {
    pub foreground: &'a mut tiny_skia::Pixmap,
    pub background: &'a crate::image_source::ImageSource,
    pub damage: Option<super::DamageRect>,
    /// 选区高亮孔洞范围（当前输出物理像素坐标）
    pub selection: Option<tiny_skia::Rect>,
    /// 候选区域悬停染色范围（当前输出物理像素坐标）
    pub hovered: Option<tiny_skia::Rect>,
    pub opaque: bool,
}
