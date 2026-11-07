//! 【界面控件】【单行输入】搜索和文件名共用 Unicode 编辑与有界剪贴板输入

use crate::ui::fonts::UiFonts;
use crate::{
    backend::window::KeyInput,
    keys::Key,
    ui::text_editor::{EditEvent, TextEditor},
};
use cosmic_text::SwashCache;
use tiny_skia::{Pixmap, Rect};

pub struct SingleLine {
    editor: TextEditor,
    limit: usize,
    offset: f32,
}

impl SingleLine {
    /// 【界面控件】【单行构造】用 text 初始化不超过 limit 字节的编辑器，fonts 用于排版；返回输入控件
    pub fn new(text: &str, fonts: &mut UiFonts, limit: usize) -> Self {
        let text = normalized(text, limit);
        fonts.ensure_text(&text);
        let mut editor = TextEditor::new(&text, fonts);
        editor.single_line(fonts);
        Self {
            editor,
            limit,
            offset: 0.0,
        }
    }

    /// 【界面控件】【字段内容】返回当前文字，单行输入不包含换行
    pub fn text(&self) -> String {
        self.editor.text()
    }

    /// 【界面控件】【字段替换】用 text 替换内容，fonts 用于排版；select_all 决定是否选中全部文字
    pub fn set(&mut self, text: &str, fonts: &mut UiFonts, select_all: bool) {
        let text = normalized(text, self.limit);
        fonts.ensure_text(&text);
        self.editor = TextEditor::new(&text, fonts);
        self.editor.single_line(fonts);
        self.offset = 0.0;
        if select_all {
            self.editor.select_all(fonts);
        }
    }

    /// 【界面控件】【全文选择】选中字段全部文字，fonts 用于排版；不修改内容
    pub fn select_all(&mut self, fonts: &mut UiFonts) {
        self.editor.select_all(fonts);
    }

    /// 【界面控件】【有界插入】规范 text 并插入选区，fonts 用于排版；返回文字是否改变
    fn insert(&mut self, text: &str, fonts: &mut UiFonts) -> bool {
        let remaining = self.limit.saturating_sub(
            self.text()
                .len()
                .saturating_sub(self.editor.selected_text().map_or(0, |text| text.len())),
        );
        let text = normalized(text, remaining);
        fonts.ensure_text(&text);
        self.editor.insert(fonts, &text)
    }

    /// 【界面控件】【单行按键】处理 input 的编辑和剪贴板操作，fonts 用于排版；返回是否需要重绘
    pub fn key(&mut self, fonts: &mut UiFonts, input: &KeyInput) -> bool {
        if matches!(input.key, Some(Key::Enter | Key::Tab)) {
            return false;
        }
        let mut input = input.clone();
        let remaining = self.limit.saturating_sub(
            self.text()
                .len()
                .saturating_sub(self.editor.selected_text().map_or(0, |text| text.len())),
        );
        input.text = normalized(&input.text, remaining);
        fonts.ensure_text(&input.text);
        match self.editor.key(fonts, &input) {
            EditEvent::None => false,
            EditEvent::Copy(text) | EditEvent::Cut(text) => {
                match crate::utils::copy_to_clipboard(&text) {
                    Ok(()) if input.key == Some(Key::Char('x')) => {
                        self.editor.delete_selection(fonts);
                    }
                    Ok(()) => {}
                    Err(error) => log::error!("【界面控件】【单行复制】复制失败: {error}"),
                }
                true
            }
            EditEvent::Paste => {
                crate::utils::paste_from_clipboard().is_some_and(|text| self.insert(&text, fonts))
            }
            EditEvent::Redraw | EditEvent::Changed => true,
        }
    }

    /// 【界面控件】【单行鼠标】根据 position 选择文字，drag 指定拖动，fonts 用于精确字符命中
    pub fn pointer(&mut self, fonts: &mut UiFonts, position: (f64, f64), drag: bool) {
        self.editor.pointer(
            fonts,
            (position.0 + self.offset as f64, position.1),
            drag,
            1,
        );
    }

    /// 【界面控件】【单行绘制】把字段绘制到 canvas 的 rect，scale 为设备比例，focused 控制光标
    pub fn draw(
        &mut self,
        fonts: &mut UiFonts,
        cache: &mut SwashCache,
        canvas: &mut Pixmap,
        rect: Rect,
        scale: f32,
        focused: bool,
    ) {
        self.editor
            .draw_single_line(fonts, cache, canvas, rect, scale, focused, &mut self.offset);
    }
}

/// 【界面控件】【输入规范】将 text 中的换行转为空格并删除其他控制字符，limit 限制 UTF-8 字节；返回有效单行文字
fn normalized(text: &str, limit: usize) -> String {
    let mut result = String::new();
    for ch in text.chars().filter_map(|ch| match ch {
        '\r' | '\n' | '\t' => Some(' '),
        ch if ch.is_control() => None,
        ch => Some(ch),
    }) {
        if result.len() + ch.len_utf8() > limit {
            break;
        }
        result.push(ch);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【单行测试】【Unicode 边界】验证输入限制与控制字符处理，不截断字素编码
    #[test]
    fn normalization_bounds_utf8_and_replaces_lines() {
        assert_eq!(normalized("汉字", 4), "汉");
        assert_eq!(normalized("a\nb\tc\0d", 99), "a b cd");
    }
}
