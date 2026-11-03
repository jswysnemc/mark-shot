//! 【文字标注】【外观数据】背景和字体独立于正文颜色，预览比例不改变原始点字号

use tiny_skia::Color;

#[derive(Clone, Debug, PartialEq)]
pub struct TextAppearance {
    pub family: String,
    pub background: Color,
    pub size_scale: f32,
    pub wrap_width: Option<f32>,
}

impl Default for TextAppearance {
    /// 【文字标注】【默认外观】返回原版 Sans Serif 与透明背景，采用原图比例且没有强制换行宽度
    fn default() -> Self {
        Self {
            family: "Sans Serif".into(),
            background: Color::TRANSPARENT,
            size_scale: 1.0,
            wrap_width: None,
        }
    }
}

impl TextAppearance {
    /// 【文字标注】【比例映射】按 scale 映射背景内边距与换行宽度，返回新外观，不改变背景颜色和字体家族
    pub fn scaled(&self, scale: f32) -> Self {
        Self {
            size_scale: self.size_scale * scale,
            wrap_width: self.wrap_width.map(|width| width * scale),
            ..self.clone()
        }
    }
}
