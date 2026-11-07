//! 【OCR 翻译】【分栏几何】采用原版 620、480 像素断点，文字绘制与输入共用同一矩形

use super::super::titlebar_layout::rect;
use cosmic_text::{Buffer, FontSystem};
use tiny_skia::Rect;

pub(crate) struct Layout {
    pub actions: Rect,
    pub caption: Option<Rect>,
    pub language: Rect,
    pub language_text: Rect,
    pub arrow: Rect,
    pub translate: Rect,
    pub source: Rect,
    pub splitter: Rect,
    pub pane: Rect,
    pub header: Rect,
    pub copy: Rect,
    pub undo: Option<Rect>,
    pub frame: Rect,
    pub editor: Rect,
    pub notice: Option<Rect>,
    pub notice_text: Option<Buffer>,
    pub progress: Option<Rect>,
    pub horizontal: bool,
}

impl Layout {
    /// 【OCR 翻译】【完整排列】按 size、分栏比例、译文历史、反馈、提示和 busy 生成几何，fonts 只测量提示，返回布局
    #[cfg(test)]
    pub fn new(
        size: (f32, f32),
        ratio: f32,
        undo: bool,
        copy_label: &str,
        notice: &str,
        busy: bool,
        fonts: &mut FontSystem,
    ) -> Self {
        Self::localized(
            size,
            ratio,
            undo,
            copy_label,
            notice,
            busy,
            fonts,
            crate::ui::language::Language::English,
        )
    }

    /// 【OCR 翻译】【语言排列】按 size、ratio、undo、copy_label、notice、busy、fonts 和 language 返回原版语言几何，提示使用完整本地化文字
    pub fn localized(
        size: (f32, f32),
        ratio: f32,
        undo: bool,
        copy_label: &str,
        notice: &str,
        busy: bool,
        fonts: &mut FontSystem,
        language: crate::ui::language::Language,
    ) -> Self {
        let (width, height) = size;
        let actions = rect(8.0, 37.0, width - 16.0, 30.0);
        let caption_width = if language == crate::ui::language::Language::Chinese {
            48.0
        } else {
            95.0
        };
        let caption = (width >= 480.0).then(|| rect(8.0, 37.0, caption_width, 30.0));
        let language_left = if caption.is_some() {
            16.0 + caption_width
        } else {
            8.0
        };
        let language = rect(language_left, 37.0, width - 108.0 - language_left, 30.0);
        let horizontal = width >= 620.0;
        let body = rect(8.0, 71.0, width - 16.0, height - 97.0);
        let extent = if horizontal {
            body.width()
        } else {
            body.height()
        };
        let available = extent - 8.0;
        let minimum_source = if horizontal { 40.0 } else { 50.0 };
        let minimum_translation = if horizontal { 162.0 } else { 96.0 };
        let source_extent = (available * ratio).round().clamp(
            minimum_source,
            (available - minimum_translation).max(minimum_source),
        );
        let (source, splitter, pane) = if horizontal {
            (
                rect(body.x(), body.y(), source_extent, body.height()),
                rect(body.x() + source_extent, body.y(), 8.0, body.height()),
                rect(
                    body.x() + source_extent + 8.0,
                    body.y(),
                    available - source_extent,
                    body.height(),
                ),
            )
        } else {
            (
                rect(body.x(), body.y(), body.width(), source_extent),
                rect(body.x(), body.y() + source_extent, body.width(), 8.0),
                rect(
                    body.x(),
                    body.y() + source_extent + 8.0,
                    body.width(),
                    available - source_extent,
                ),
            )
        };
        let copy_width = super::super::labels::copy_width(copy_label);
        let notice_text = super::notice::shape(fonts, notice, pane.width());
        let notice_height = super::notice::height(notice_text.as_ref());
        let notice = (notice_height > 0.0).then(|| {
            rect(
                pane.x(),
                pane.bottom() - notice_height,
                pane.width(),
                notice_height,
            )
        });
        let bottom = notice.map_or(pane.bottom(), |notice| notice.y() - 2.0);
        let progress = busy.then(|| rect(pane.x(), bottom - 3.0, pane.width(), 3.0));
        let bottom = progress.map_or(bottom, |progress| progress.y() - 2.0);
        let frame = rect(
            pane.x(),
            pane.y() + 23.0,
            pane.width(),
            bottom - pane.y() - 23.0,
        );
        Self {
            actions,
            caption,
            language,
            language_text: rect(
                language.x() + 11.0,
                language.y() + 6.0,
                language.width() - 64.0,
                18.0,
            ),
            arrow: rect(language.right() - 25.0, language.y() + 3.0, 24.0, 24.0),
            translate: rect(width - 100.0, 37.0, 92.0, 30.0),
            source,
            splitter,
            pane,
            header: rect(pane.x(), pane.y(), pane.width(), 21.0),
            copy: rect(pane.right() - copy_width, pane.y(), copy_width, 21.0),
            undo: undo.then(|| rect(pane.right() - copy_width - 32.0, pane.y(), 28.0, 21.0)),
            editor: rect(
                frame.x() + 5.0,
                frame.y() + 5.0,
                frame.width() - 10.0,
                frame.height() - 10.0,
            ),
            frame,
            notice,
            notice_text,
            progress,
            horizontal,
        }
    }
}

#[cfg(test)]
mod tests;
