//! 【截图历史】【窗口布局】紧凑设置、操作栏与窄窗口分区

pub(super) use crate::ui::file_picker::layout::contains;
use tiny_skia::Rect;

pub(super) const ROW: f32 = 62.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Control {
    Close,
    Enabled,
    Minus,
    Plus,
    Refresh,
    Copy,
    Edit,
    Pin,
    Save,
    Delete,
    Clear,
    ConfirmCancel,
    ConfirmClear,
}

impl Control {
    /// 【截图历史】【按钮文本】返回按钮名称；不使用字符图标或表情符号
    pub fn label(self) -> &'static str {
        match self {
            Self::Close => "Close",
            Self::Enabled => "Recording",
            Self::Minus => "-",
            Self::Plus => "+",
            Self::Refresh => "Refresh",
            Self::Copy => "Copy",
            Self::Edit => "Edit",
            Self::Pin => "Pin",
            Self::Save => "Save as",
            Self::Delete => "Delete",
            Self::Clear => "Clear",
            Self::ConfirmCancel => "Cancel",
            Self::ConfirmClear => "Clear",
        }
    }
}

pub(super) struct Layout {
    pub size: (f32, f32),
    pub search: Rect,
    pub search_text: Rect,
    pub limit: Rect,
    pub limit_text: Rect,
    pub list: Rect,
    pub preview: Rect,
    pub buttons: Vec<(Control, Rect)>,
    pub confirm: Rect,
    pub confirm_buttons: [(Control, Rect); 2],
}

/// 【截图历史】【布局矩形】返回 x、y、width、height 的区域；调用方保证正尺寸
pub(super) fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_xywh(x, y, width, height).unwrap()
}

impl Layout {
    /// 【截图历史】【自适应布局】根据 size 生成布局；窄于六百二十像素时把预览移到列表下方
    pub fn new(size: (u32, u32)) -> Self {
        let (width, height) = (size.0.max(360) as f32, size.1.max(360) as f32);
        let body = rect(8.0, 136.0, width - 16.0, height - 206.0);
        let (list, preview) = if width >= 620.0 {
            let list_width = (body.width() * 0.34).clamp(204.0, 300.0);
            (
                rect(body.x(), body.y(), list_width, body.height()),
                rect(
                    body.x() + list_width + 8.0,
                    body.y(),
                    body.width() - list_width - 8.0,
                    body.height(),
                ),
            )
        } else {
            let list_height = (body.height() * 0.54).max(ROW);
            (
                rect(body.x(), body.y(), body.width(), list_height),
                rect(
                    body.x(),
                    body.y() + list_height + 6.0,
                    body.width(),
                    body.height() - list_height - 6.0,
                ),
            )
        };
        let mut buttons = vec![
            (Control::Close, rect(width - 60.0, 6.0, 52.0, 26.0)),
            (Control::Enabled, rect(8.0, 42.0, 112.0, 28.0)),
            (Control::Minus, rect(158.0, 42.0, 26.0, 28.0)),
            (Control::Plus, rect(238.0, 42.0, 26.0, 28.0)),
            (Control::Refresh, rect(width - 80.0, 42.0, 72.0, 28.0)),
        ];
        let button_width = ((width - 36.0) / 6.0).min(78.0);
        for (index, control) in [
            Control::Copy,
            Control::Edit,
            Control::Pin,
            Control::Save,
            Control::Delete,
            Control::Clear,
        ]
        .into_iter()
        .enumerate()
        {
            buttons.push((
                control,
                rect(
                    8.0 + index as f32 * (button_width + 4.0),
                    height - 62.0,
                    button_width,
                    28.0,
                ),
            ));
        }
        let cw = (width - 32.0).min(420.0);
        let cx = (width - cw) * 0.5;
        let cy = (height - 152.0) * 0.5;
        Self {
            size: (width, height),
            search: rect(8.0, 80.0, width - 16.0, 28.0),
            search_text: rect(14.0, 82.0, width - 28.0, 24.0),
            limit: rect(188.0, 42.0, 46.0, 28.0),
            limit_text: rect(193.0, 44.0, 36.0, 24.0),
            list,
            preview,
            buttons,
            confirm: rect(cx, cy, cw, 152.0),
            confirm_buttons: [
                (
                    Control::ConfirmCancel,
                    rect(cx + cw - 160.0, cy + 110.0, 68.0, 28.0),
                ),
                (
                    Control::ConfirmClear,
                    rect(cx + cw - 84.0, cy + 110.0, 72.0, 28.0),
                ),
            ],
        }
    }

    /// 【截图历史】【控件命中】查找 position 对应控件；清空确认弹层独占操作区域
    pub fn control(&self, position: (f64, f64), confirmation: bool) -> Option<Control> {
        let buttons = if confirmation {
            self.confirm_buttons.as_slice()
        } else {
            &self.buttons
        };
        buttons
            .iter()
            .find(|(_, rect)| contains(*rect, position))
            .map(|(control, _)| *control)
    }

    /// 【截图历史】【可见行数】返回列表视口中的完整行数；最少显示一行
    pub fn rows(&self) -> usize {
        (self.list.height() / ROW).floor().max(1.0) as usize
    }

    /// 【截图历史】【调整大小】返回 position 是否在窗口右下角的调整区域
    pub fn resize_at(&self, position: (f64, f64)) -> bool {
        position.0 >= self.size.0 as f64 - 18.0 && position.1 >= self.size.1 as f64 - 18.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【截图历史测试】【窄屏布局】数量、刷新和操作按钮不重叠，列表与预览不超出窗口
    #[test]
    fn compact_layout_keeps_all_controls_accessible() {
        for width in [360, 480, 620, 760, 1280] {
            let layout = Layout::new((width, 360));
            assert!(layout.preview.width() > 0.0 && layout.preview.height() > 0.0);
            assert!(
                layout
                    .buttons
                    .iter()
                    .all(|(_, rect)| rect.right() <= width as f32 && rect.bottom() <= 360.0)
            );
            assert!(
                layout.limit.right()
                    < layout
                        .buttons
                        .iter()
                        .find(|(control, _)| *control == Control::Plus)
                        .unwrap()
                        .1
                        .left()
            );
        }
    }
}
