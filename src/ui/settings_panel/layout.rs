//! 【属性面板】【紧凑换行】控件绘制与命中共享行列矩形，字号放大后仍限制在屏幕内

use super::SettingsPanel;
use crate::ui::panel::PanelItem;
use tiny_skia::Rect;

impl SettingsPanel {
    /// 【属性面板】【排版同步】按 available_width 排列控件，返回几何是否变化，不修改控件值或标注
    pub fn arrange(&mut self, available_width: f32) -> bool {
        let old = (self.size, self.cells.clone());
        self.cells.clear();
        if let Some(layout) = self.text_layout {
            self.size = layout.size;
        } else {
            let padding = crate::theme::size::padding();
            let row = crate::ui::toolbar::metrics::properties_height();
            let height = row * crate::config::get().settings_panel.item_height;
            let limit = available_width.max(160.0) - padding;
            let mut x = padding;
            let mut y = (row - height) / 2.0;
            let mut right = padding;
            for widget in self.widgets {
                let width = widget.size().min((limit - padding).max(1.0));
                if x > padding && x + width > limit {
                    x = padding;
                    y += row;
                }
                self.cells
                    .push(Rect::from_xywh(x, y, width, height).unwrap());
                right = right.max(x + width);
                x += width + widget.trailing_padding();
            }
            self.size = (right + padding, y + height + (row - height) / 2.0);
        }
        let changed = old != (self.size, self.cells.clone());
        self.dirty |= changed;
        changed
    }
}
