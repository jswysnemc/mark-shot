//! 【界面控件】【局部样式】编辑器自行保存字号、行高和可选颜色，不覆盖共享主题

use crate::theme::Rgba;

#[derive(Clone, Copy)]
pub struct EditorStyle {
    pub size: f32,
    pub line_height: f32,
    pub colors: Option<[Rgba; 4]>,
}

impl Default for EditorStyle {
    /// 【界面控件】【默认样式】返回现有字号和行高，颜色在绘制时读取共享主题，无参数
    fn default() -> Self {
        let size = crate::theme::font::label();
        Self {
            size,
            line_height: size * 1.65,
            colors: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::text_editor::TextEditor;
    use cosmic_text::Edit;

    /// 【编辑样式测试】【字体隔离】局部字号、行高和选择颜色不改变默认编辑器或撤销历史，无参数、无返回值
    #[test]
    fn explicit_style_is_local_and_survives_edits() {
        let mut fonts = crate::fonts::bundled();
        let colors = [Rgba(20, 40, 60, 255); 4];
        let mut local = TextEditor::new_with_style(
            "source",
            &mut fonts,
            EditorStyle {
                size: 15.0,
                line_height: 20.0,
                colors: Some(colors),
            },
        );
        let default = TextEditor::new("other", &mut fonts);
        assert_eq!(
            local.editor.with_buffer(|b| b.metrics()),
            cosmic_text::Metrics::new(15.0, 20.0)
        );
        assert_eq!(
            default.editor.with_buffer(|b| b.metrics()).font_size,
            crate::theme::font::label()
        );
        assert!(default.style.colors.is_none());
        local.insert(&mut fonts, "!");
        local.undo(&mut fonts);
        assert_eq!(local.text(), "source");
        assert_eq!(local.style.colors, Some(colors));
        assert_eq!(
            local.editor.with_buffer(|b| b.metrics()),
            cosmic_text::Metrics::new(15.0, 20.0)
        );
    }
}
