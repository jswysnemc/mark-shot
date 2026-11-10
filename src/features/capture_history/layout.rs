//! 【截图历史】【窗口布局】紧凑操作栏、可调水平分区与底部记录开关

pub(super) use crate::ui::file_picker::layout::contains;
use tiny_skia::Rect;

pub(super) const ROW: f32 = 40.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Control {
    Close,
    Enabled,
    Copy,
    Edit,
    Pin,
    Save,
    Delete,
    Clear,
}

impl Control {
    /// 【截图历史】【按钮文本】返回按钮名称；不使用字符图标或表情符号
    pub fn label(self) -> &'static str {
        match self {
            Self::Close => "Close",
            Self::Enabled => "Keep screenshot history",
            Self::Copy => "Copy",
            Self::Edit => "Edit",
            Self::Pin => "Pin",
            Self::Save => "Save As",
            Self::Delete => "Delete",
            Self::Clear => "Clear History",
        }
    }
}

pub(super) struct Layout {
    pub size: (f32, f32),
    pub summary: Rect,
    pub status: Option<Rect>,
    pub list: Rect,
    pub preview: Rect,
    pub splitter: Rect,
    pub buttons: Vec<(Control, Rect)>,
}

/// 【截图历史】【布局矩形】返回 x、y、width、height 的区域；调用方保证正尺寸
pub(super) fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_xywh(x, y, width, height).unwrap()
}

impl Layout {
    /// 【截图历史】【默认布局】按 size 创建紧凑水平分区，返回没有状态提示的布局
    pub fn new(size: (u32, u32)) -> Self {
        Self::with_options(size, 190.0, 0.0)
    }

    /// 【截图历史】【窗口重排】根据 size、left_width 和 status_height 返回布局；保留两侧最小分区及状态行留白
    pub fn with_options(size: (u32, u32), left_width: f32, status_height: f32) -> Self {
        let (width, height) = (size.0.max(440) as f32, size.1.max(320) as f32);
        let inset = if status_height > 0.0 {
            status_height + 6.0
        } else {
            0.0
        };
        let body = rect(8.0, 68.0, width - 16.0, (height - 108.0 - inset).max(100.0));
        let left_width = super::splitter::clamp(left_width, body.width());
        let list = rect(body.x(), body.y(), left_width, body.height());
        let preview = rect(
            body.x() + left_width - 1.0,
            body.y(),
            body.width() - left_width + 1.0,
            body.height(),
        );
        let splitter = rect(body.x() + left_width - 3.0, body.y(), 5.0, body.height());
        let mut buttons = vec![
            (Control::Close, rect(width - 36.0, 5.0, 28.0, 26.0)),
            (
                Control::Enabled,
                rect(10.0, height - 29.0 - inset, 163.0, 18.0),
            ),
            (
                Control::Clear,
                rect(width - 127.0, height - 33.0 - inset, 119.0, 26.0),
            ),
        ];
        let widths = [68.0, 66.0, 62.0, 82.0, 72.0];
        let mut x = 8.0;
        for (control, button_width) in [
            Control::Copy,
            Control::Edit,
            Control::Pin,
            Control::Save,
            Control::Delete,
        ]
        .into_iter()
        .zip(widths)
        {
            buttons.push((control, rect(x, 36.0, button_width, 26.0)));
            x += button_width + 4.0;
        }
        Self {
            size: (width, height),
            summary: rect(10.0, 5.0, width - 54.0, 26.0),
            status: (status_height > 0.0).then(|| {
                rect(
                    12.0,
                    height - 10.0 - status_height,
                    width - 24.0,
                    status_height,
                )
            }),
            list,
            preview,
            splitter,
            buttons,
        }
    }

    /// 【截图历史】【控件命中】查找 position 对应父窗口控件，不包含独立对话框区域
    pub fn control(&self, position: (f64, f64)) -> Option<Control> {
        self.buttons
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
mod tests;
