//! 【文字识别】【结果布局】普通窗口的紧凑布局与命中范围

use tiny_skia::Rect;

use super::{
    titlebar_layout::{Titlebar, rect},
    window::View,
};

pub(super) const INITIAL_SIZE: (u32, u32) = (640, 520);
pub(super) const MINIMUM_SIZE: (u32, u32) = (380, 340);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Control {
    View(View),
    Copy,
    TableCopy,
    Undo,
    Close,
    More,
    TranslationToggle,
    Translate,
    TranslationCopy,
    TranslationUndo,
    LanguageMenu,
    AlwaysOnTop,
}

pub(super) struct Layout {
    pub size: (f32, f32),
    pub titlebar: Titlebar,
    pub body: Rect,
    pub editor_frame: Rect,
    pub editor: Rect,
    pub table_body: Rect,
    pub table_copy: Rect,
    pub table_header: super::table_header_layout::Header,
    pub close: Rect,
    pub more: Rect,
    pub copy: Rect,
    pub undo: Option<Rect>,
    pub grip: Rect,
    pub translation: Option<super::translation::layout::Layout>,
}

/// 【文字识别】【区域命中】判断 position 是否位于 rect 内；边界使用左闭右开规则
pub(super) fn contains(rect: Rect, position: (f64, f64)) -> bool {
    let (x, y) = (position.0 as f32, position.1 as f32);
    x >= rect.left() && x < rect.right() && y >= rect.top() && y < rect.bottom()
}

impl Layout {
    /// 【文字识别】【初始布局】按 size 与 has_table 返回未编辑文字页布局，反馈及页面变化由 with_state 更新
    #[cfg(test)]
    pub fn new(size: (u32, u32), has_table: bool) -> Self {
        Self::with_state(
            size,
            has_table,
            true,
            View::Text,
            "Copy",
            "Copy",
            false,
            false,
        )
    }

    /// 【文字识别】【状态布局】按 size、可选页签、view、两区反馈、原文 undo 和 status 返回控件几何，窄页签保持原始宽度
    #[cfg(test)]
    pub fn with_state(
        size: (u32, u32),
        has_table: bool,
        has_image: bool,
        view: View,
        source_label: &str,
        table_label: &str,
        undo: bool,
        status: bool,
    ) -> Self {
        Self::localized(
            size,
            has_table,
            has_image,
            view,
            source_label,
            table_label,
            undo,
            status,
            crate::ui::language::Language::English,
        )
    }

    /// 【文字识别】【语言布局】按 size、可选页签、view、两区反馈、undo、status 和 language 返回完整控件几何，正文数据不参与翻译
    pub fn localized(
        size: (u32, u32),
        has_table: bool,
        has_image: bool,
        view: View,
        source_label: &str,
        table_label: &str,
        undo: bool,
        status: bool,
        language: crate::ui::language::Language,
    ) -> Self {
        let (width, height) = (
            size.0.max(MINIMUM_SIZE.0) as f32,
            size.1.max(MINIMUM_SIZE.1) as f32,
        );
        let titlebar =
            Titlebar::localized(width, has_table, has_image, view, source_label, language);
        let body = rect(8.0, 37.0, width - 16.0, height - 63.0);
        let undo = (view == View::Text && undo).then(|| rect(width - 36.0, 37.0, 28.0, 20.0));
        let top = if undo.is_some() { 59.0 } else { 37.0 };
        let editor_frame = rect(8.0, top, width - 16.0, body.bottom() - top);
        let editor_viewport = rect(11.0, top + 3.0, width - 22.0, editor_frame.height() - 6.0);
        let table_width = super::labels::table_copy_width(table_label, language);
        let table_copy = rect(width - 8.0 - table_width, 39.0, table_width, 30.0);
        Self {
            size: (width, height),
            body,
            editor_frame,
            editor: editor_viewport,
            table_body: rect(8.0, 75.0, width - 16.0, body.height() - 38.0),
            table_copy,
            table_header: super::table_header_layout::Header::new(table_copy, language),
            close: titlebar.slots[3].unwrap(),
            more: titlebar.slots[1].unwrap(),
            copy: titlebar.copy,
            undo,
            grip: rect(
                if status {
                    width - 26.0
                } else {
                    ((width - 18.0) / 2.0).floor() + 1.0
                },
                height - 22.0,
                18.0,
                18.0,
            ),
            titlebar,
            translation: None,
        }
    }

    /// 【文字识别】【正文边距】返回编辑器视区扣除两像素文档边距后的文字矩形，无参数；输入位置与绘制共用此区域
    pub fn text_rect(&self) -> Rect {
        rect(
            self.editor.x() + 2.0,
            self.editor.y() + 2.0,
            self.editor.width() - 4.0,
            self.editor.height() - 4.0,
        )
    }

    /// 【文字识别】【控件命中】按 position 与 view 查找页面和共用控件；原生菜单输入由独立表面处理
    pub fn control(&self, position: (f64, f64), view: View) -> Option<Control> {
        if view == View::Text {
            if self.titlebar.slots[0].is_some_and(|rect| contains(rect, position)) {
                return Some(Control::TranslationToggle);
            }
            if let Some(layout) = &self.translation {
                for (control, rect) in [
                    (Control::Translate, layout.translate),
                    (Control::TranslationCopy, layout.copy),
                    (Control::LanguageMenu, layout.arrow),
                ] {
                    if (control == Control::TranslationCopy || contains(layout.actions, position))
                        && contains(rect, position)
                    {
                        return Some(control);
                    }
                }
                if layout.undo.is_some_and(|rect| contains(rect, position)) {
                    return Some(Control::TranslationUndo);
                }
            }
        }
        if view == View::Table && contains(self.table_copy, position) {
            return Some(Control::TableCopy);
        }
        if contains(self.close, position) {
            return Some(Control::Close);
        }
        if self.titlebar.slots[2].is_some_and(|rect| contains(rect, position)) {
            return Some(Control::AlwaysOnTop);
        }
        if contains(self.more, position) {
            return Some(Control::More);
        }
        if contains(self.copy, position) {
            return Some(Control::Copy);
        }
        if view == View::Text && self.undo.is_some_and(|rect| contains(rect, position)) {
            return Some(Control::Undo);
        }
        self.titlebar.tab_at(position).map(Control::View)
    }

    /// 【文字识别】【调整大小】判断 position 是否位于当前页脚尺寸入口，返回命中结果
    pub fn resize_at(&self, position: (f64, f64)) -> bool {
        contains(self.grip, position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【结果布局测试】【窄窗口】验证可见页签视区与复制按钮不重叠，正文始终保持有效尺寸，无参数、无返回值
    #[test]
    fn narrow_layout_keeps_controls_separate() {
        for width in [320, 430, 760, 1200] {
            let layout = Layout::new((width, 240), true);
            assert!(layout.titlebar.viewport.right() < layout.copy.left());
            assert!(layout.body.width() > 0.0 && layout.editor.height() > 0.0);
        }
    }
}
