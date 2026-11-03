//! 【文字事务】【文档格式】当前文档字体独立于工具和标注样式，局部撤销后不在普通面板刷新中覆盖

use crate::{
    editor::{
        EditorState,
        text_session::{self, PREVIEW_ID},
    },
    features::text::TextAppearance,
    types::{Annotation, AnnotationShape},
};
use cosmic_text::Edit;
use tiny_skia::Color;

#[derive(Clone, PartialEq)]
pub struct Font {
    pub points: f32,
    pub family: String,
    pub bold: bool,
    pub italic: bool,
}

impl Font {
    /// 【文字事务】【字体读取】从 annotation 读取当前文字格式，返回与正文内容无关的字体；非文字对象拒绝使用
    pub fn read(annotation: &Annotation) -> Self {
        let AnnotationShape::Text {
            font_size,
            bold,
            italic,
            appearance,
            ..
        } = &annotation.shape
        else {
            panic!("text font requires a text annotation")
        };
        Self {
            points: *font_size,
            family: appearance.family.clone(),
            bold: *bold,
            italic: *italic,
        }
    }

    /// 【文字事务】【字体应用】将当前字体写入 annotation，只改变字号和字形，不覆盖正文、颜色、位置或换行宽度，无返回值
    pub fn apply(&self, annotation: &mut Annotation) {
        if let AnnotationShape::Text {
            font_size,
            bold,
            italic,
            appearance,
            ..
        } = &mut annotation.shape
        {
            *font_size = self.points;
            *bold = self.bold;
            *italic = self.italic;
            appearance.family.clone_from(&self.family);
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Source {
    pub font: Font,
    color: Color,
    outline: Option<Color>,
    appearance: TextAppearance,
}

impl Source {
    /// 【文字事务】【样式来源】从 annotation 提取不含正文的样式，返回用于属性变化比较的轻量记录
    pub fn read(annotation: &Annotation) -> Self {
        let AnnotationShape::Text {
            outline,
            appearance,
            ..
        } = &annotation.shape
        else {
            unreachable!()
        };
        Self {
            font: Font::read(annotation),
            color: annotation.color,
            outline: *outline,
            appearance: appearance.clone(),
        }
    }

    /// 【文字事务】【当前来源】按 state 会话取得持久对象或工具默认样式，返回独立来源；没有会话时返回空值
    fn current(state: &EditorState) -> Option<Self> {
        let session = state.text_session.as_ref()?;
        if let Some(id) = session.original_id {
            return state
                .annotations
                .iter()
                .find(|annotation| annotation.id == id)
                .map(Self::read);
        }
        let tools = &state.tool_settings;
        let mut source = session.source.clone();
        source.font = Font {
            points: tools.font_size,
            family: tools.text_family.clone(),
            bold: tools.bold,
            italic: tools.italic,
        };
        source.color = tools.color;
        source.outline = tools.outline.then_some(tools.outline_color);
        source.appearance.family.clone_from(&tools.text_family);
        source.appearance.background = tools.text_background;
        Some(source)
    }
}

/// 【文字事务】【格式同步】将 state 属性变化应用到预览；force 对应原版显式字号确认，dirty 返回重绘范围，不覆盖已经撤销的文档格式
pub fn sync(state: &mut EditorState, force: bool, dirty: &mut u32) {
    let Some(source) = Source::current(state) else {
        return;
    };
    let session = state.text_session.as_mut().unwrap();
    if !force && source == session.source {
        return;
    }
    let reformat = force || source.font != session.source.font;
    if reformat {
        let before = Font::read(&session.working);
        let editor = state.text.editors.get_mut(&PREVIEW_ID).unwrap();
        // 1. 【文字事务】【全篇格式】原版格式按钮清除字符选区，撤销恢复字体但不恢复按钮操作前的选区
        editor.set_selection(cosmic_text::Selection::None);
        let has_text = editor.with_buffer(|buffer| {
            buffer.lines.len() > 1
                || buffer
                    .lines
                    .first()
                    .is_some_and(|line| !line.text().is_empty())
        });
        if has_text {
            session
                .history
                .record_font(before, source.font.clone(), editor.cursor());
        }
        source.font.apply(&mut session.working);
    }
    // 2. 【文字事务】【来源隔离】颜色和背景跟随属性来源，文档字体只在实际格式入口变化时更新
    session.working.color = source.color;
    if let AnnotationShape::Text {
        outline,
        appearance,
        ..
    } = &mut session.working.shape
    {
        *outline = source.outline;
        let family = appearance.family.clone();
        *appearance = source.appearance.clone();
        appearance.family = family;
    }
    session.source = source;
    session.properties_hidden = false;
    text_session::refresh(state, dirty);
}

/// 【文字事务】【提交字体】按 state 最后实际属性来源准备统一成稿；文档格式撤销不改原版提交元数据，无参数、无返回值
pub fn prepare_commit(state: &mut EditorState) {
    if let Some(session) = state.text_session.as_mut() {
        session.source.font.apply(&mut session.working);
    }
}
