//! 【文字识别】【表格网格绘制】原版调色板、数字表头、单元格选择与独立裁切

use tiny_skia::Rect;

use super::{
    palette::Palette,
    table::Table,
    table_geometry::RowGeometry,
    table_grid_layout::{CELL_FONT, Grid, HEADER_FONT},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    ui::controls,
};

pub(super) struct DrawGrid<'a> {
    pub table: &'a Table,
    pub rows: &'a RowGeometry,
    pub grid: &'a Grid,
    pub active: Option<(usize, usize)>,
    pub editing: bool,
    pub scroll: (f32, f32),
}

impl DrawGrid<'_> {
    /// 【文字识别】【单元格绘制】用 painter、colors 绘制可见单元格；表头、空白和滚动保留区不接收正文像素
    pub fn cells(&self, painter: &mut Painter<'_>, colors: Palette) {
        painter.clipped_scene(Some(self.grid.viewport), |painter| {
            for row in self
                .rows
                .visible(self.scroll.1, self.grid.viewport.height())
            {
                for column in self.grid.visible_columns(self.scroll.0) {
                    let (left, width) = self.grid.column(column);
                    let x = self.grid.viewport.x() + left - self.scroll.0;
                    let y = self.grid.viewport.y() + self.rows.top(row) - self.scroll.1;
                    let height = self.rows.height(row);
                    let cell = Rect::from_xywh(x, y, width - 1.0, height - 1.0).unwrap();
                    let selected = self.active == Some((row, column));
                    if selected {
                        painter.fill_rounded(cell, 0.0, colors.accent);
                    }
                    if !selected || !self.editing {
                        painter.clipped_scene(Some(cell), |painter| {
                            let text = controls::fit_label(
                                painter,
                                &self.table.rows[row][column],
                                CELL_FONT,
                                width - 7.0,
                            );
                            painter.text(
                                Rect::from_xywh(x + 3.0, y, width - 7.0, height - 1.0).unwrap(),
                                &text,
                                TextStyle::new(
                                    CELL_FONT,
                                    if selected {
                                        colors.on_accent
                                    } else {
                                        colors.text
                                    },
                                ),
                            );
                        });
                    }
                    painter.fill_rounded(
                        Rect::from_xywh(x + width - 1.0, y, 1.0, height).unwrap(),
                        0.0,
                        colors.border,
                    );
                    painter.fill_rounded(
                        Rect::from_xywh(x, y + height - 1.0, width, 1.0).unwrap(),
                        0.0,
                        colors.border,
                    );
                }
            }
        });
    }

    /// 【文字识别】【表头绘制】用独立字体 painter 和 colors 绘制数字列头、行号和角区，滚动时保持固定表头位置
    pub fn headers(&self, painter: &mut Painter<'_>, colors: Palette) {
        painter.fill_rounded(self.grid.corner, 0.0, colors.alternate);
        // 1. 【文字识别】【横向表头】滚动到部分列时，文字与边线只绘入横向表头，不覆盖角区
        painter.clipped_scene(Some(self.grid.horizontal_header), |painter| {
            for column in self.grid.visible_columns(self.scroll.0) {
                let (left, width) = self.grid.column(column);
                let x = self.grid.viewport.x() + left - self.scroll.0;
                let header =
                    Rect::from_xywh(x, 1.0, width, self.grid.horizontal_header.height()).unwrap();
                painter.fill_rounded(header, 0.0, colors.alternate);
                let weight = if self.active.is_some_and(|(_, current)| current == column) {
                    cosmic_text::Weight::BOLD
                } else {
                    cosmic_text::Weight::NORMAL
                };
                painter.text(
                    header,
                    &(column + 1).to_string(),
                    TextStyle::new(HEADER_FONT, colors.muted)
                        .center()
                        .weight(weight),
                );
                painter.fill_rounded(
                    Rect::from_xywh(header.right() - 1.0, header.y(), 1.0, header.height())
                        .unwrap(),
                    0.0,
                    colors.border,
                );
                painter.fill_rounded(
                    Rect::from_xywh(header.x(), header.bottom() - 1.0, header.width(), 1.0)
                        .unwrap(),
                    0.0,
                    colors.border,
                );
            }
        });
        // 2. 【文字识别】【纵向表头】仅实际数据行绘制底色；剩余空白沿用窗口基色
        painter.clipped_scene(Some(self.grid.vertical_header), |painter| {
            for row in self
                .rows
                .visible(self.scroll.1, self.grid.viewport.height())
            {
                let y = self.grid.viewport.y() + self.rows.top(row) - self.scroll.1;
                let header = Rect::from_xywh(
                    1.0,
                    y,
                    self.grid.vertical_header.width(),
                    self.rows.height(row),
                )
                .unwrap();
                painter.fill_rounded(header, 0.0, colors.alternate);
                let weight = if self.active.is_some_and(|(current, _)| current == row) {
                    cosmic_text::Weight::BOLD
                } else {
                    cosmic_text::Weight::NORMAL
                };
                painter.text(
                    header,
                    &(row + 1).to_string(),
                    TextStyle::new(HEADER_FONT, colors.muted)
                        .center()
                        .weight(weight),
                );
                painter.fill_rounded(
                    Rect::from_xywh(header.right() - 1.0, header.y(), 1.0, header.height())
                        .unwrap(),
                    0.0,
                    colors.border,
                );
                painter.fill_rounded(
                    Rect::from_xywh(header.x(), header.bottom() - 1.0, header.width(), 1.0)
                        .unwrap(),
                    0.0,
                    colors.border,
                );
            }
        });
    }
}

#[cfg(test)]
mod tests;
