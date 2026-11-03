//! 【截图历史】【窗口布局】复用原版顶部操作栏、水平分隔区与底部记录开关

pub(super) use crate::ui::file_picker::layout::contains;
use tiny_skia::Rect;

pub(super) const ROW: f32 = 48.0;

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
    ConfirmCancel,
    ConfirmClear,
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
            Self::ConfirmCancel => "Cancel",
            Self::ConfirmClear => "Clear History",
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
    pub confirm: Rect,
    pub confirm_buttons: [(Control, Rect); 2],
}

/// 【截图历史】【布局矩形】返回 x、y、width、height 的区域；调用方保证正尺寸
pub(super) fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_xywh(x, y, width, height).unwrap()
}

impl Layout {
    /// 【截图历史】【默认布局】按 size 创建原版初始水平分区，返回没有状态提示的布局
    pub fn new(size: (u32, u32)) -> Self {
        Self::with_options(size, 210.0, 0.0)
    }

    /// 【截图历史】【窗口重排】根据 size、left_width 和 status_height 返回布局；保留原版最小分区及状态行留白
    pub fn with_options(size: (u32, u32), left_width: f32, status_height: f32) -> Self {
        let (width, height) = (size.0.max(440) as f32, size.1.max(320) as f32);
        let inset = if status_height > 0.0 {
            status_height + 6.0
        } else {
            0.0
        };
        let body = rect(
            12.0,
            82.0,
            width - 24.0,
            (height - 128.0 - inset).max(100.0),
        );
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
            (Control::Close, rect(width - 44.0, 10.0, 32.0, 30.0)),
            (
                Control::Enabled,
                rect(12.0, height - 34.0 - inset, 163.0, 18.0),
            ),
            (
                Control::Clear,
                rect(width - 131.0, height - 40.0 - inset, 119.0, 30.0),
            ),
        ];
        let widths = action_widths(width - 40.0);
        let mut x = 12.0;
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
            buttons.push((control, rect(x, 46.0, button_width, 30.0)));
            x += button_width + 4.0;
        }
        let cw = (width - 32.0).min(420.0);
        let cx = (width - cw) * 0.5;
        let cy = (height - 152.0) * 0.5;
        Self {
            size: (width, height),
            summary: rect(12.0, 10.0, width - 62.0, 30.0),
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

/// 【截图历史】【操作栏分配】按 available 及原版最小尺寸返回五个按钮宽度，伸展空间平均分配并保留整数像素
fn action_widths(available: f32) -> [f32; 5] {
    let minimum = [78.0, 79.0, 78.0, 86.0, 79.0];
    let mut result = [0.0; 5];
    let mut fixed = [false; 5];
    let mut left = available;
    loop {
        let count = fixed.iter().filter(|value| !**value).count();
        if count == 0 {
            return result;
        }
        let share = left / count as f32;
        let mut changed = false;
        for index in 0..5 {
            if !fixed[index] && minimum[index] > share {
                result[index] = minimum[index];
                fixed[index] = true;
                left -= result[index];
                changed = true;
            }
        }
        if !changed {
            let mut assigned: f32 = 0.0;
            for index in 0..5 {
                if !fixed[index] {
                    let next = assigned + share;
                    result[index] = next.round() - assigned.round();
                    assigned = next;
                }
            }
            return result;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【截图历史测试】【Qt 参考】读取真实窗口的冻结样本，返回原版几何及输入行为，不模拟原版布局
    fn qt_reference() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/qt-history-window.json"
        ))
        .unwrap()
    }

    /// 【截图历史测试】【矩形比较】比较 actual 与 expected 中的逻辑矩形，name 标记具体控件，无返回值
    fn same_rect(name: &str, actual: Rect, expected: &serde_json::Value) {
        let actual = [actual.x(), actual.y(), actual.width(), actual.height()];
        let expected: Vec<f32> = expected
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_f64().unwrap() as f32)
            .collect();
        assert_eq!(actual.as_slice(), expected, "{name}");
    }

    /// 【截图历史测试】【原版布局】列表与预览保持水平分区，图像操作位于上方，不允许额外数量或刷新按钮，无参数、无返回值
    #[test]
    fn qt_history_geometry_and_control_inventory_match() {
        let reference = qt_reference();
        for case in reference["layouts"].as_array().unwrap() {
            let size = case["size"].as_array().unwrap();
            let layout = Layout::new((
                size[0].as_u64().unwrap() as u32,
                size[1].as_u64().unwrap() as u32,
            ));
            same_rect("history list", layout.list, &case["list"]);
            same_rect("history preview", layout.preview, &case["preview"]);
            assert_eq!(
                layout.buttons.len(),
                case["actions"].as_array().unwrap().len() + 3
            );
            for (control, field) in [
                (Control::Close, "close"),
                (Control::Enabled, "enabled"),
                (Control::Clear, "clear"),
            ] {
                let bounds = layout
                    .buttons
                    .iter()
                    .find(|(kind, _)| *kind == control)
                    .unwrap()
                    .1;
                same_rect(field, bounds, &case[field]);
            }
            for (control, expected) in [
                Control::Copy,
                Control::Edit,
                Control::Pin,
                Control::Save,
                Control::Delete,
            ]
            .into_iter()
            .zip(case["actions"].as_array().unwrap())
            {
                let bounds = layout
                    .buttons
                    .iter()
                    .find(|(kind, _)| *kind == control)
                    .unwrap()
                    .1;
                same_rect(control.label(), bounds, &expected["rect"]);
            }
        }
        let first = &reference["layouts"][0]["rows"];
        let spacing = first[1]["rect"][1].as_f64().unwrap() - first[0]["rect"][1].as_f64().unwrap();
        assert_eq!(ROW, spacing as f32);
    }

    /// 【截图历史测试】【窄屏布局】原版最小窗口仍保留水平列表、预览与单行按钮，无参数、无返回值
    #[test]
    fn compact_layout_keeps_all_controls_accessible() {
        for width in [440, 480, 620, 760, 1280] {
            let layout = Layout::new((width, 320));
            assert!(layout.preview.width() > 0.0 && layout.preview.height() > 0.0);
            assert!(
                layout
                    .buttons
                    .iter()
                    .all(|(_, rect)| rect.right() <= width as f32 && rect.bottom() <= 320.0)
            );
            assert_eq!(layout.list.y(), layout.preview.y());
            assert!(layout.list.width() >= 184.0 && layout.preview.width() >= 180.0);
        }
    }

    /// 【截图历史测试】【状态几何】原版状态行占据十九像素并增加六像素间距，各控件矩形严格匹配，无参数、无返回值
    #[test]
    fn qt_history_status_and_splitter_geometry_match() {
        let reference: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/qt-history-details.json"
        ))
        .unwrap();
        let layout = Layout::with_options((800, 520), 210.0, 19.0);
        same_rect("status", layout.status.unwrap(), &reference["status"]);
        same_rect("status list", layout.list, &reference["status_list"]);
        for (control, field) in [
            (Control::Enabled, "status_enabled"),
            (Control::Clear, "status_clear"),
        ] {
            same_rect(
                field,
                layout
                    .buttons
                    .iter()
                    .find(|(kind, _)| *kind == control)
                    .unwrap()
                    .1,
                &reference[field],
            );
        }
        for case in reference["splitters"].as_array().unwrap() {
            let layout =
                Layout::with_options((800, 520), case["requested"].as_f64().unwrap() as f32, 0.0);
            same_rect("splitter", layout.splitter, &case["handle"]);
            same_rect("splitter list", layout.list, &case["list"]);
            same_rect("splitter preview", layout.preview, &case["preview"]);
        }
    }
}
