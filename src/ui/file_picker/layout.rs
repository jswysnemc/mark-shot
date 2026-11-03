//! 【文件选择】【布局】紧凑保存对话框与控件命中

use tiny_skia::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Home,
    Up,
    Hidden,
    NewFolder,
    Cancel,
    Save,
    ConfirmCancel,
    ConfirmReplace,
}

pub struct Layout {
    pub panel: Rect,
    pub path: Rect,
    pub path_text: Rect,
    pub list: Rect,
    pub filename: Rect,
    pub filename_text: Rect,
    pub buttons: Vec<(Control, Rect)>,
    pub confirm: Rect,
    pub confirm_buttons: [(Control, Rect); 2],
}

/// 【文件选择】【区域命中】判断 position 是否在 rect 内；使用左闭右开的边界
pub fn contains(rect: Rect, position: (f64, f64)) -> bool {
    position.0 >= rect.left() as f64
        && position.0 < rect.right() as f64
        && position.1 >= rect.top() as f64
        && position.1 < rect.bottom() as f64
}

impl Layout {
    /// 【文件选择】【自适应区域】按 size 创建对话框，不超过窗口边界；返回目录、字段和按钮区域
    pub fn new(size: (u32, u32)) -> Self {
        let (width, height) = (size.0.max(300) as f32, size.1.max(300) as f32);
        let (w, h) = ((width - 16.0).min(660.0), (height - 16.0).min(440.0));
        let (x, y) = ((width - w) * 0.5, (height - h) * 0.5);
        let rect = |x, y, w, h| Rect::from_xywh(x, y, w, h).unwrap();
        let mut buttons = Vec::new();
        for (control, dx, bw) in [
            (Control::Home, 12.0, 48.0),
            (Control::Up, 64.0, 40.0),
            (Control::Hidden, 108.0, 70.0),
            (Control::NewFolder, 182.0, 88.0),
        ] {
            buttons.push((control, rect(x + dx, y + 74.0, bw, 28.0)));
        }
        buttons.push((
            Control::Cancel,
            rect(x + w - 160.0, y + h - 42.0, 68.0, 28.0),
        ));
        buttons.push((Control::Save, rect(x + w - 84.0, y + h - 42.0, 72.0, 28.0)));
        let cw = (w - 24.0).min(360.0);
        let cx = x + (w - cw) * 0.5;
        let cy = y + (h - 148.0) * 0.5;
        Self {
            panel: rect(x, y, w, h),
            path: rect(x + 12.0, y + 38.0, w - 24.0, 28.0),
            path_text: rect(x + 18.0, y + 40.0, w - 36.0, 24.0),
            list: rect(x + 12.0, y + 110.0, w - 24.0, h - 224.0),
            filename: rect(x + 58.0, y + h - 80.0, w - 70.0, 28.0),
            filename_text: rect(x + 64.0, y + h - 78.0, w - 82.0, 24.0),
            buttons,
            confirm: rect(cx, cy, cw, 148.0),
            confirm_buttons: [
                (
                    Control::ConfirmCancel,
                    rect(cx + cw - 160.0, cy + 104.0, 68.0, 28.0),
                ),
                (
                    Control::ConfirmReplace,
                    rect(cx + cw - 84.0, cy + 104.0, 72.0, 28.0),
                ),
            ],
        }
    }

    /// 【文件选择】【按钮命中】按 position 返回控件，overwrite 为真时只允许确认弹层操作
    pub fn control(&self, position: (f64, f64), overwrite: bool) -> Option<Control> {
        let buttons = if overwrite {
            self.confirm_buttons.as_slice()
        } else {
            &self.buttons
        };
        buttons
            .iter()
            .find(|(_, rect)| contains(*rect, position))
            .map(|(control, _)| *control)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【文件选择测试】【窄屏边界】导航、列表、文件名和确认按钮不超出窗口；确认状态禁止底层按钮命中
    #[test]
    fn modal_layout_and_hit_testing_remain_compact() {
        for size in [(360, 360), (480, 400), (760, 460)] {
            let layout = Layout::new(size);
            assert!(layout.list.height() > 0.0);
            assert!(layout.filename.right() < layout.panel.right());
            for &(_, rect) in &layout.buttons {
                assert!(
                    rect.left() >= 0.0
                        && rect.right() <= size.0 as f32
                        && rect.bottom() <= size.1 as f32
                );
            }
            let home = layout.buttons[0].1;
            assert_eq!(
                layout.control((home.x() as f64 + 1.0, home.y() as f64 + 1.0), true),
                None
            );
        }
    }
}
