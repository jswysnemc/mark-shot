//! 【框选翻译】【控件度量】标题和按钮使用原版十三像素字体及六十四分之一像素字宽

/// 【框选翻译】【文字宽度】按 text 和 fonts 返回固定字宽，按钮向上取整，语言列表取最近整数
pub(super) fn width(text: &str, fonts: &mut cosmic_text::FontSystem) -> f32 {
    crate::features::image_translation::shaping::prepare(fonts, text);
    crate::features::image_translation::shaping::width(fonts, text, 13.0, 18.0) as f32
}
