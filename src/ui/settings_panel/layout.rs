//! 【副工具栏】【紧凑换行】控件绘制与命中共享行列矩形，分隔线两侧保持对称紧凑间距

use super::{SettingsPanel, SettingsWidget};
use crate::ui::panel::PanelItem;
use tiny_skia::Rect;

impl SettingsPanel {
    /// 【副工具栏】【排版同步】按 available_width 排列控件，返回几何是否变化，不修改控件值或标注
    pub fn arrange(&mut self, available_width: f32) -> bool {
        let old = (self.size, self.cells.clone());
        self.cells.clear();
        if let Some(layout) = self.text_layout {
            self.size = layout.size;
        } else {
            let padding = crate::theme::size::padding().max(7.0);
            let row = crate::ui::toolbar::metrics::properties_height();
            let height = (row * crate::config::get().settings_panel.item_height).round();
            let limit = available_width.max(160.0) - padding;
            let mut x = padding;
            let mut y = ((row - height) * 0.5).round();
            let mut right = padding;
            for (index, widget) in self.widgets.iter().enumerate() {
                let width = self.widget_width(widget).min((limit - padding).max(1.0));
                // 1. 换行首项若为分隔线则不跨行产生多余空白，普通控件超宽时换至下一行
                if x > padding && x + width > limit {
                    x = padding;
                    y += row;
                }
                self.cells
                    .push(Rect::from_xywh(x, y, width, height).unwrap());
                right = right.max(x + width);
                let next_is_sep =
                    matches!(self.widgets.get(index + 1), Some(SettingsWidget::Separator));
                let gap = if next_is_sep {
                    0.0
                } else {
                    widget.trailing_padding()
                };
                x += width + gap;
            }
            self.size = (right + padding, y + height + ((row - height) * 0.5).round());
        }
        let changed = old != (self.size, self.cells.clone());
        self.dirty |= changed;
        changed
    }
}
