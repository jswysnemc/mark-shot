//! 【文字识别】【表格编辑】按需创建单元格编辑器，维护可滚动表格视区

use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::{Pixmap, Rect};

use super::{layout::contains, table::Table, table_geometry::RowGeometry};
use crate::{
    backend::wayland::window::KeyInput,
    keys::Key,
    renderer::{Painter, text::TextStyle},
    theme::{color, font},
    ui::{
        controls,
        text_editor::{EditEvent, TextEditor},
    },
};

const GUTTER: f32 = 32.0;
const HEADER: f32 = 26.0;

pub(super) struct TableView {
    pub table: Table,
    pub active: Option<(usize, usize)>,
    pub editor: Option<TextEditor>,
    pub scroll: (f32, f32),
    rows: RowGeometry,
    raster: Option<Pixmap>,
}

/// 【文字识别】【列名称】将零起点 index 转换为电子表格列名；返回 A 到 BL 等名称
fn column_name(mut index: usize) -> String {
    let mut result = Vec::new();
    loop {
        result.push((b'A' + (index % 26) as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    result.into_iter().rev().collect()
}

impl TableView {
    /// 【文字识别】【表格视区】创建 table 的编辑视区；单元格字体与排版仅在激活时创建
    pub fn new(table: Table) -> Self {
        let rows = RowGeometry::new(&table);
        Self {
            table,
            rows,
            active: None,
            editor: None,
            scroll: (0.0, 0.0),
            raster: None,
        }
    }

    /// 【文字识别】【列宽计算】根据 body 宽度和列数返回紧凑列宽；多列布局允许水平滚动
    fn column_width(&self, body: Rect) -> f32 {
        ((body.width() - GUTTER) / self.table.columns().max(1) as f32).clamp(100.0, 220.0)
    }

    /// 【文字识别】【单元格定位】将 position 映射到 body 内的行列；标题、行号与空白区域返回空值
    pub fn cell_at(&self, body: Rect, position: (f64, f64)) -> Option<(usize, usize)> {
        if !contains(body, position) {
            return None;
        }
        let (x, y) = (position.0 as f32 - body.x(), position.1 as f32 - body.y());
        if x < GUTTER || y < HEADER {
            return None;
        }
        let column = ((x - GUTTER + self.scroll.0) / self.column_width(body)).floor() as usize;
        let row = self.rows.at(y - HEADER + self.scroll.1)?;
        (column < self.table.columns()).then_some((row, column))
    }

    /// 【文字识别】【单元格区域】返回 row、column 在 body 本地坐标中的文字区域
    fn text_rect(&self, body: Rect, row: usize, column: usize) -> Rect {
        let width = self.column_width(body);
        Rect::from_xywh(
            GUTTER + column as f32 * width - self.scroll.0 + 6.0,
            HEADER + self.rows.top(row) - self.scroll.1 + 5.0,
            width - 12.0,
            self.rows.height(row) - 10.0,
        )
        .unwrap()
    }

    /// 【文字识别】【激活单元格】激活 row、column 并初始化编辑器；fonts 用于文字选择与排版
    fn activate(&mut self, fonts: &mut FontSystem, row: usize, column: usize) {
        if self.active == Some((row, column)) {
            return;
        }
        self.sync();
        self.active = Some((row, column));
        self.editor = Some(TextEditor::new(&self.table.rows[row][column], fonts));
    }

    /// 【文字识别】【单元格同步】将活动编辑器内容写回表格；无活动单元格时不修改
    pub fn sync(&mut self) {
        if let (Some((row, column)), Some(editor)) = (self.active, &mut self.editor) {
            self.table.set_cell(row, column, editor.text());
            if self.rows.update(row, &self.table.rows[row]) {
                editor.reset_scroll();
            }
        }
    }

    /// 【文字识别】【表格鼠标】在 body 内按 position 定位和编辑单元格；drag、clicks 控制文字选择
    pub fn pointer(
        &mut self,
        fonts: &mut FontSystem,
        body: Rect,
        position: (f64, f64),
        drag: bool,
        clicks: u8,
    ) {
        let cell = if drag {
            self.active
        } else {
            self.cell_at(body, position)
        };
        let Some((row, column)) = cell else {
            return;
        };
        self.activate(fonts, row, column);
        let rect = self.text_rect(body, row, column);
        if let Some(editor) = &mut self.editor {
            editor.resize(fonts, rect.width(), rect.height());
            editor.pointer(
                fonts,
                (
                    position.0 - f64::from(body.x() + rect.x()),
                    position.1 - f64::from(body.y() + rect.y()),
                ),
                drag,
                clicks,
            );
        }
    }

    /// 【文字识别】【表格按键】编辑活动单元格；Tab 和 Enter 移动到相邻单元格，返回剪贴板或重绘请求
    pub fn key(&mut self, fonts: &mut FontSystem, body: Rect, input: &KeyInput) -> EditEvent {
        if !input.mods.ctrl
            && matches!(input.key, Some(Key::Tab | Key::Enter))
            && !(input.key == Some(Key::Enter) && input.mods.shift)
        {
            let Some((row, column)) = self.active else {
                return EditEvent::None;
            };
            let index = row * self.table.columns() + column;
            let next = if input.key == Some(Key::Tab) {
                if input.mods.shift {
                    index.saturating_sub(1)
                } else {
                    (index + 1).min(self.table.rows.len() * self.table.columns() - 1)
                }
            } else {
                ((row + 1).min(self.table.rows.len() - 1)) * self.table.columns() + column
            };
            self.activate(
                fonts,
                next / self.table.columns(),
                next % self.table.columns(),
            );
            self.ensure_visible(body);
            if let Some(editor) = &mut self.editor {
                editor.select_all(fonts);
            }
            return EditEvent::Redraw;
        }
        let event = self
            .editor
            .as_mut()
            .map_or(EditEvent::None, |editor| editor.key(fonts, input));
        self.sync();
        event
    }

    /// 【文字识别】【表格滚动】按 delta 移动表格视区；根据 body 大小限制两个方向的滚动范围
    pub fn scroll(&mut self, body: Rect, delta: (f64, f64)) {
        let max_x = (self.column_width(body) * self.table.columns() as f32 + GUTTER - body.width())
            .max(0.0);
        let max_y = (self.rows.total_height() + HEADER - body.height()).max(0.0);
        self.scroll.0 = (self.scroll.0 + delta.0 as f32).clamp(0.0, max_x);
        self.scroll.1 = (self.scroll.1 + delta.1 as f32).clamp(0.0, max_y);
    }

    /// 【文字识别】【单元格可见】根据 body 大小滚动到活动单元格；保留已在视区内的位置
    fn ensure_visible(&mut self, body: Rect) {
        let Some((row, column)) = self.active else {
            return;
        };
        let width = self.column_width(body);
        let (x, y) = (column as f32 * width, self.rows.top(row));
        let visible_width = body.width() - GUTTER;
        let visible_height = body.height() - HEADER;
        self.scroll.0 = self.scroll.0.min(x).max(x + width - visible_width);
        self.scroll.1 = self
            .scroll
            .1
            .min(y)
            .max(y + self.rows.height(row) - visible_height);
        self.scroll(body, (0.0, 0.0));
    }

    /// 【文字识别】【表格绘制】在 canvas 的 body 内绘制可见单元格；fonts、cache、scale 负责清晰文字与分数缩放
    pub fn draw(
        &mut self,
        fonts: &mut FontSystem,
        cache: &mut SwashCache,
        canvas: &mut Pixmap,
        body: Rect,
        scale: f32,
        focused: bool,
    ) {
        self.scroll(body, (0.0, 0.0));
        let (width, height) = (
            (body.width() * scale).ceil() as u32,
            (body.height() * scale).ceil() as u32,
        );
        if self
            .raster
            .as_ref()
            .is_none_or(|raster| raster.width() != width || raster.height() != height)
        {
            self.raster = Pixmap::new(width, height);
        }
        let column_width = self.column_width(body);
        let visible_rows = self.rows.visible(self.scroll.1, body.height() - HEADER);
        let first_column = (self.scroll.0 / column_width).floor() as usize;
        let last_column = ((self.scroll.0 + body.width() - GUTTER) / column_width).ceil() as usize;
        let Some(raster) = self.raster.as_mut() else {
            return;
        };
        raster.fill(color::field().color());
        {
            let mut painter = Painter::new(raster)
                .with_scale(scale)
                .with_text(fonts, cache);
            for row in visible_rows.clone() {
                let row_height = self.rows.height(row);
                for column in first_column..last_column.min(self.table.columns()) {
                    let (x, y) = (
                        GUTTER + column as f32 * column_width - self.scroll.0,
                        HEADER + self.rows.top(row) - self.scroll.1,
                    );
                    let rect =
                        Rect::from_xywh(x + 1.0, y + 1.0, column_width - 2.0, row_height - 2.0)
                            .unwrap();
                    if self.active == Some((row, column)) {
                        painter.fill_rounded(rect, 3.0, color::active().with_alpha(42));
                    }
                    if self.active != Some((row, column)) {
                        let text = &self.table.rows[row][column];
                        let label = controls::fit_label(
                            &mut painter,
                            text,
                            font::label(),
                            column_width - 12.0,
                        );
                        painter.text(
                            Rect::from_xywh(x + 6.0, y, column_width - 12.0, row_height).unwrap(),
                            &label,
                            TextStyle::new(font::label(), color::foreground()),
                        );
                    }
                    painter.fill_rounded(
                        Rect::from_xywh(x, y + row_height - 1.0, column_width, 1.0).unwrap(),
                        0.0,
                        color::border(),
                    );
                    painter.fill_rounded(
                        Rect::from_xywh(x + column_width - 1.0, y, 1.0, row_height).unwrap(),
                        0.0,
                        color::border(),
                    );
                }
            }
        }
        if let (Some((row, column)), Some(editor)) = (self.active, &mut self.editor) {
            let rect = Rect::from_xywh(
                GUTTER + column as f32 * column_width - self.scroll.0 + 6.0,
                HEADER + self.rows.top(row) - self.scroll.1 + 5.0,
                column_width - 12.0,
                self.rows.height(row) - 10.0,
            )
            .unwrap();
            editor.draw(fonts, cache, raster, rect, scale, focused);
        }
        {
            let mut painter = Painter::new(raster)
                .with_scale(scale)
                .with_text(fonts, cache);
            painter.fill_rounded(
                Rect::from_xywh(0.0, 0.0, body.width(), HEADER).unwrap(),
                0.0,
                color::panel(),
            );
            painter.fill_rounded(
                Rect::from_xywh(0.0, 0.0, GUTTER, body.height()).unwrap(),
                0.0,
                color::panel(),
            );
            for column in first_column..last_column.min(self.table.columns()) {
                let x = GUTTER + column as f32 * column_width - self.scroll.0;
                painter.text(
                    Rect::from_xywh(x, 0.0, column_width, HEADER).unwrap(),
                    &column_name(column),
                    TextStyle::new(font::small(), color::muted()).center(),
                );
            }
            for row in visible_rows {
                let y = HEADER + self.rows.top(row) - self.scroll.1;
                painter.text(
                    Rect::from_xywh(0.0, y, GUTTER, self.rows.height(row)).unwrap(),
                    &(row + 1).to_string(),
                    TextStyle::new(font::small(), color::muted()).center(),
                );
            }
        }
        canvas.draw_pixmap(
            (body.x() * scale).round() as i32,
            (body.y() * scale).round() as i32,
            raster.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            tiny_skia::Transform::identity(),
            None,
        );
    }
}
