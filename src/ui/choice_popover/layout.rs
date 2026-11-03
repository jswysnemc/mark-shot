//! 【选择面板】【网格布局】根据可用空间计算列数、可见行和条目命中

use tiny_skia::Rect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChoiceLayout {
    pub count: usize,
    pub columns: usize,
    pub rows: usize,
    pub visible_rows: usize,
    pub cell_size: f32,
    pub size: (f32, f32),
    pub padding: f32,
    pub gap: f32,
    pub grid_top: f32,
    pub title_height: f32,
    pub label_height: f32,
}

impl ChoiceLayout {
    /// 【选择面板】【尺寸计算】按 count 和 max_size 计算紧凑网格；返回布局，超出高度的条目通过滚动访问
    pub fn new(count: usize, max_size: (f32, f32)) -> Self {
        let cfg = &crate::config::get().choice_picker;
        let padding = cfg.padding.max(0.0);
        let gap = cfg.gap.max(0.0);
        let requested_cell = cfg.cell_size.max(1.0);
        let available_width = (max_size.0 - padding * 2.0).max(1.0);
        let fit_columns = ((available_width + gap) / (requested_cell + gap))
            .floor()
            .max(1.0) as usize;
        let columns = cfg.columns.max(1).min(count.max(1)).min(fit_columns);
        let cell_size = requested_cell.min(
            ((available_width - gap * columns.saturating_sub(1) as f32) / columns as f32).max(1.0),
        );
        let rows = count.div_ceil(columns);
        let title_height = cfg.title_height.max(1.0);
        let label_height = cfg.label_height.max(1.0);
        let grid_top = padding + title_height + gap;
        let available_height = (max_size.1 - grid_top - label_height - padding).max(cell_size);
        let fit_rows = ((available_height + gap) / (cell_size + gap))
            .floor()
            .max(1.0) as usize;
        let visible_rows = fit_rows.min(rows);
        let grid_height =
            visible_rows as f32 * cell_size + visible_rows.saturating_sub(1) as f32 * gap;
        Self {
            count,
            columns,
            rows,
            visible_rows,
            cell_size,
            size: (
                padding * 2.0 + columns as f32 * cell_size + columns.saturating_sub(1) as f32 * gap,
                grid_top + grid_height + label_height + padding,
            ),
            padding,
            gap,
            grid_top,
            title_height,
            label_height,
        }
    }

    /// 【选择面板】【条目矩形】计算 index 在 first_row 滚动位置的面板本地矩形；不可见或越界时返回空值
    pub fn item_rect(&self, index: usize, first_row: usize) -> Option<Rect> {
        let row = index / self.columns;
        if index >= self.count || row < first_row || row >= first_row + self.visible_rows {
            return None;
        }
        Rect::from_xywh(
            self.padding + (index % self.columns) as f32 * (self.cell_size + self.gap),
            self.grid_top + (row - first_row) as f32 * (self.cell_size + self.gap),
            self.cell_size,
            self.cell_size,
        )
    }

    /// 【选择面板】【条目命中】根据面板本地 position 和 first_row 返回条目索引；留白、标题和越界位置返回空值
    pub fn item_at(&self, position: (f32, f32), first_row: usize) -> Option<usize> {
        let x = position.0 - self.padding;
        let y = position.1 - self.grid_top;
        let stride = self.cell_size + self.gap;
        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
            return None;
        }
        let column = (x / stride).floor() as usize;
        let row = (y / stride).floor() as usize;
        if column >= self.columns
            || row >= self.visible_rows
            || x % stride > self.cell_size
            || y % stride > self.cell_size
        {
            return None;
        }
        let index = (first_row + row) * self.columns + column;
        (index < self.count).then_some(index)
    }
}
