//! 【界面控件】【单行输入】搜索和文件名共用 Unicode 编辑与有界剪贴板输入

use crate::ui::fonts::UiFonts;
use crate::{
    backend::window::KeyInput,
    keys::Key,
    types::text_input::{ChangeCause, Edit, Focus, Snapshot},
    ui::text_editor::{EditEvent, InputArea, InputMethodUpdate, TextEditor},
};
use cosmic_text::SwashCache;
use tiny_skia::{Pixmap, Rect};

pub struct SingleLine {
    editor: TextEditor,
    limit: usize,
    offset: f32,
}

impl SingleLine {
    /// 【界面控件】【居中数值】按 text、fonts 创建紧凑居中输入框，返回与静态数值字号一致的控件
    pub fn number(text: &str, fonts: &mut UiFonts) -> Self {
        let mut editor = TextEditor::new_single_line_with_style(
            text,
            fonts,
            crate::ui::text_editor::EditorStyle {
                size: 12.0,
                line_height: 26.0,
                colors: None,
            },
        );
        editor.center(fonts);
        Self {
            editor,
            limit: 64,
            offset: 0.0,
        }
    }
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

    /// 【界面控件】【密码反馈】无参数，返回有界遮蔽字符，不泄露原始文字
    pub fn masked_text(&self) -> String {
        "•".repeat(self.text().chars().count().min(16))
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

    /// 【界面控件】【输入身份】返回单行编辑器的稳定协议身份，无参数
    pub fn input_identity(&self) -> u64 {
        self.editor.input_identity()
    }

    /// 【界面控件】【候选取消】清理当前未确认预编辑候选，无参数，返回是否存在候选变化
    pub fn cancel_preedit(&mut self) -> bool {
        self.editor.cancel_preedit()
    }

    /// 【界面控件】【输入快照】按 fonts、rect 及协议参数生成单行光标与周围文本快照，无变化返回空值
    pub fn input_snapshot(
        &mut self,
        fonts: &mut UiFonts,
        rect: Rect,
        focus: Focus,
        target_id: u64,
        previous: Option<&Snapshot>,
        cause: ChangeCause,
    ) -> Option<Snapshot> {
        self.editor.input_snapshot(
            fonts,
            InputArea {
                rect,
                offset: self.offset,
            },
            focus,
            target_id,
            previous,
            cause,
        )
    }

    /// 【界面控件】【输入法提交】按单行容量约束与字体准备应用 edit 批次，返回候选与正文变更状态
    pub fn input_method(&mut self, fonts: &mut UiFonts, mut edit: Edit) -> InputMethodUpdate {
        let selected = self.editor.selected_text().map_or(0, |text| text.len());
        let replaced = edit.replacement.map_or(0, |value| value.length);
        let removed = selected.saturating_add(replaced);
        let remaining = self
            .limit
            .saturating_sub(self.text().len().saturating_sub(removed));
        edit.commit = normalized(&edit.commit, remaining);
        fonts.ensure_text(&edit.commit);
        fonts.ensure_text(&edit.preedit.text);
        self.editor.input_method(fonts, edit)
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

    /// 【单行测试】【中文输入法】验证预编辑候选与确认提交遵循字节上限且替换已有全文选区
    #[test]
    fn single_line_accepts_ime_preedit_and_commit_within_limit() {
        use crate::types::text_input::Preedit;

        let mut fonts = UiFonts::with_system_family("Sans Serif");
        let mut field = SingleLine::new("default", &mut fonts, 12);
        field.select_all(&mut fonts);
        let preedit = field.input_method(
            &mut fonts,
            Edit {
                preedit: Preedit {
                    text: "zhong".into(),
                    cursor: Some((5, 5)),
                },
                commit: String::new(),
                replacement: None,
            },
        );
        assert!(preedit.accepted && preedit.changed && preedit.text_changed);
        let next_preedit = field.input_method(
            &mut fonts,
            Edit {
                preedit: Preedit {
                    text: "zhongwen".into(),
                    cursor: Some((8, 8)),
                },
                commit: String::new(),
                replacement: None,
            },
        );
        assert!(next_preedit.accepted && next_preedit.changed && !next_preedit.text_changed);
        let committed = field.input_method(
            &mut fonts,
            Edit {
                preedit: Preedit::default(),
                commit: "中文输入法".into(),
                replacement: None,
            },
        );
        assert!(committed.accepted && committed.text_changed);
        assert_eq!(field.text(), "中文输入");
    }
}
