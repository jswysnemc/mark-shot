//! 【文字识别】【结果布局】普通窗口的紧凑布局与命中范围

use tiny_skia::Rect;

use super::{export::Format, window::View};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Control {
    View(View),
    Copy,
    Undo,
    Close,
    Format(Format),
}

pub(super) struct Layout {
    pub size: (f32, f32),
    pub body: Rect,
    pub editor: Rect,
    pub close: Rect,
    pub copy: Rect,
    pub undo: Option<Rect>,
    pub tabs: Vec<(View, Rect)>,
    pub formats: Vec<(Format, Rect)>,
}

/// 【文字识别】【区域命中】判断 position 是否位于 rect 内；边界使用左闭右开规则
pub(super) fn contains(rect: Rect, position: (f64, f64)) -> bool {
    let (x, y) = (position.0 as f32, position.1 as f32);
    x >= rect.left() && x < rect.right() && y >= rect.top() && y < rect.bottom()
}

impl Layout {
    /// 【文字识别】【自适应布局】按 size 与 has_table 生成控件区域；窄窗口隐藏撤销按钮但保留快捷键
    pub fn new(size: (u32, u32), has_table: bool) -> Self {
        let (width, height) = (size.0.max(300) as f32, size.1.max(180) as f32);
        let rect = |x, y, width, height| Rect::from_xywh(x, y, width, height).unwrap();
        let mut tabs = vec![
            (View::Text, rect(8.0, 42.0, 56.0, 28.0)),
            (View::Image, rect(68.0, 42.0, 60.0, 28.0)),
        ];
        if has_table {
            tabs.push((View::Table, rect(132.0, 42.0, 58.0, 28.0)));
        }
        let copy = rect(width - 80.0, 42.0, 72.0, 28.0);
        let formats = [Format::Tsv, Format::Markdown, Format::Csv, Format::Html]
            .into_iter()
            .enumerate()
            .map(|(index, format)| {
                (
                    format,
                    rect(width - 190.0, 78.0 + index as f32 * 30.0, 182.0, 28.0),
                )
            })
            .collect();
        Self {
            size: (width, height),
            body: rect(8.0, 78.0, width - 16.0, height - 106.0),
            editor: rect(18.0, 88.0, width - 36.0, height - 126.0),
            close: rect(width - 60.0, 6.0, 52.0, 26.0),
            copy,
            undo: (width >= 430.0).then(|| rect(width - 148.0, 42.0, 60.0, 28.0)),
            tabs,
            formats,
        }
    }

    /// 【文字识别】【控件命中】按 position 与 menu_open 查找控件；菜单优先于正文和工具栏
    pub fn control(&self, position: (f64, f64), menu_open: bool) -> Option<Control> {
        if menu_open {
            for &(format, rect) in &self.formats {
                if contains(rect, position) {
                    return Some(Control::Format(format));
                }
            }
        }
        if contains(self.close, position) {
            return Some(Control::Close);
        }
        if contains(self.copy, position) {
            return Some(Control::Copy);
        }
        if self.undo.is_some_and(|rect| contains(rect, position)) {
            return Some(Control::Undo);
        }
        self.tabs
            .iter()
            .find(|(_, rect)| contains(*rect, position))
            .map(|(view, _)| Control::View(*view))
    }

    /// 【文字识别】【调整大小】判断 position 是否位于右下角拖动区域
    pub fn resize_at(&self, position: (f64, f64)) -> bool {
        position.0 >= f64::from(self.size.0 - 18.0) && position.1 >= f64::from(self.size.1 - 18.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【结果布局测试】【窄窗口】验证全部页签与复制按钮不重叠，正文始终保持有效尺寸
    #[test]
    fn narrow_layout_keeps_controls_separate() {
        for width in [320, 430, 760, 1200] {
            let layout = Layout::new((width, 240), true);
            assert!(layout.tabs.last().unwrap().1.right() < layout.copy.left());
            assert!(layout.body.width() > 0.0 && layout.editor.height() > 0.0);
            if let Some(undo) = layout.undo {
                assert!(layout.tabs.last().unwrap().1.right() < undo.left());
            }
        }
    }
}
