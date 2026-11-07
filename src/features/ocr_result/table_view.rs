//! 【文字识别】【表格编辑】组合稳定网格尺寸、可滚动视区及按需单元格编辑器

use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::{Pixmap, Rect};

use super::{
    layout::contains,
    palette::Palette,
    table::Table,
    table_geometry::RowGeometry,
    table_grid_draw::DrawGrid,
    table_grid_layout::{Columns, Grid},
    table_scroll_input::Interaction,
    table_scroll_model::Wheel,
};
use crate::{
    renderer::Painter,
    theme::Rgba,
    ui::{controls, text_editor::TextEditor},
};

pub(super) struct TableView {
    pub table: Table,
    pub active: Option<(usize, usize)>,
    pub editor: Option<TextEditor>,
    pub editor_offset: f32,
    pub scroll: (f32, f32),
    pub(super) wheel: Wheel,
    pub(super) bars: Interaction,
    rows: RowGeometry,
    columns: Columns,
    header_fonts: FontSystem,
    header_cache: SwashCache,
    raster: Option<Pixmap>,
}

impl TableView {
    /// 【文字识别】【表格视区】用 table、fonts 创建稳定内容列宽和原版固定行高，返回独立视区；字体与颜色不改变其他窗口
    pub fn new(table: Table, fonts: &mut FontSystem) -> Self {
        let rows = RowGeometry::new(&table);
        let mut header_fonts = crate::fonts::bundled();
        let family =
            crate::fonts::text_faces::family(&mut header_fonts, "DejaVu Sans", false, false);
        crate::fonts::text_faces::family(&mut header_fonts, &family, true, false);
        header_fonts.db_mut().set_sans_serif_family(family);
        let columns = Columns::new(&table, fonts, &mut header_fonts);
        Self {
            table,
            rows,
            columns,
            header_fonts,
            header_cache: SwashCache::new(),
            active: None,
            editor: None,
            editor_offset: 0.0,
            scroll: (0.0, 0.0),
            wheel: Wheel::default(),
            bars: Interaction::default(),
            raster: None,
        }
    }

    /// 【文字识别】【共享网格】按 body 返回表格本地网格，供绘制、滚动和编辑命中复用
    pub(super) fn grid(&self, body: Rect) -> Grid {
        self.columns
            .grid((body.width(), body.height()), self.table.rows.len())
    }

    /// 【文字识别】【单元格定位】将 position 映射到 body 内的行列；表头、滚动保留区和空白返回空值
    pub fn cell_at(&self, body: Rect, position: (f64, f64)) -> Option<(usize, usize)> {
        let grid = self.grid(body);
        let local = (
            position.0 - f64::from(body.x()),
            position.1 - f64::from(body.y()),
        );
        if grid
            .horizontal_scroll
            .is_some_and(|rect| contains(rect, local))
            || grid
                .vertical_scroll
                .is_some_and(|rect| contains(rect, local))
        {
            return None;
        }
        if !contains(grid.viewport, local) {
            return None;
        }
        let column = grid.column_at(local.0 as f32 - grid.viewport.x() + self.scroll.0)?;
        let row = self
            .rows
            .at(local.1 as f32 - grid.viewport.y() + self.scroll.1)?;
        Some((row, column))
    }

    /// 【文字识别】【单元格区域】按 body、row、column 返回本地文字区域，尺寸与原版固定单元格一致
    pub(super) fn text_rect(&self, body: Rect, row: usize, column: usize) -> Rect {
        let grid = self.grid(body);
        let (left, width) = grid.column(column);
        Rect::from_xywh(
            grid.viewport.x() + left - self.scroll.0 + 3.0,
            grid.viewport.y() + self.rows.top(row) - self.scroll.1 + 4.0,
            width - 7.0,
            self.rows.height(row) - 9.0,
        )
        .unwrap()
    }

    /// 【文字识别】【单元格同步】将活动编辑器内容写回表格，无返回值；原版行列尺寸不随编辑自动变化
    pub fn sync(&mut self) {
        if let (Some((row, column)), Some(editor)) = (self.active, &mut self.editor) {
            self.table.set_cell(row, column, editor.text());
        }
    }

    /// 【文字识别】【单元格可见】按 body 滚动到活动单元格，保留已在视区内的位置，无返回值
    pub(super) fn ensure_visible(&mut self, body: Rect) {
        let Some((row, column)) = self.active else {
            return;
        };
        let grid = self.grid(body);
        let (x, width) = grid.column(column);
        let y = self.rows.top(row);
        self.scroll = grid.clamp_scroll((
            self.scroll.0.min(x).max(x + width - grid.viewport.width()),
            self.scroll
                .1
                .min(y)
                .max(y + self.rows.height(row) - grid.viewport.height()),
        ));
        self.normalize_scroll(body);
    }

    /// 【文字识别】【表格绘制】用 fonts、cache 将表格绘入 canvas 的 body，scale 控制缓冲比例，focused 控制编辑光标，无返回值
    pub fn draw(
        &mut self,
        fonts: &mut FontSystem,
        cache: &mut SwashCache,
        canvas: &mut Pixmap,
        body: Rect,
        scale: f32,
        focused: bool,
    ) {
        self.normalize_scroll(body);
        let grid = self.grid(body);
        let bars = self.scroll_bars(body);
        let editor_rect = self
            .active
            .map(|(row, column)| self.text_rect(body, row, column));
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
        let Some(raster) = self.raster.as_mut() else {
            return;
        };
        raster.fill(tiny_skia::Color::TRANSPARENT);
        let colors = Palette::current();
        let bounds = Rect::from_xywh(0.0, 0.0, body.width(), body.height()).unwrap();
        let drawing = DrawGrid {
            table: &self.table,
            rows: &self.rows,
            grid: &grid,
            active: self.active,
            editing: self.editor.is_some(),
            scroll: self.scroll,
        };
        // 1. 【文字识别】【正文绘制】单元格和编辑器共享裁切视区，滚动时不会把正文绘入表头
        {
            let mut painter = Painter::new(raster)
                .with_scale(scale)
                .with_text(fonts, cache);
            controls::frame(&mut painter, bounds, colors.base, colors.border, 6.0);
            drawing.cells(&mut painter, colors);
        }
        if let (Some(rect), Some(editor)) = (editor_rect, &mut self.editor) {
            editor.draw_single_line_clipped(
                fonts,
                cache,
                raster,
                rect,
                grid.viewport,
                scale,
                focused,
                &mut self.editor_offset,
            );
        }
        {
            let mut painter = Painter::new(raster)
                .with_scale(scale)
                .with_text(&mut self.header_fonts, &mut self.header_cache);
            drawing.headers(&mut painter, colors);
        }
        super::table_scroll_draw::draw(
            &mut Painter::new(raster).with_scale(scale),
            &bars,
            &self.bars,
            colors,
        );
        // 2. 【文字识别】【圆角边界】表头按矩形绘制后统一裁切圆角，保持外边框和透明角区
        if let Some(path) =
            crate::renderer::paths::rounded_rect_path(&bounds, 6.0, true, true, true, true)
            && let Some(mut mask) = tiny_skia::Mask::new(width, height)
        {
            mask.fill_path(
                &path,
                tiny_skia::FillRule::Winding,
                true,
                tiny_skia::Transform::from_scale(scale, scale),
            );
            raster.apply_mask(&mask);
        }
        controls::frame(
            &mut Painter::new(raster).with_scale(scale),
            bounds,
            Rgba(0, 0, 0, 0),
            colors.border,
            6.0,
        );
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
