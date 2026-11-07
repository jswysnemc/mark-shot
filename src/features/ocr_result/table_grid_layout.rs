//! 【文字识别】【表格网格布局】内容列宽、末列拉伸及固定表头视区，不依赖画布像素

use std::{collections::HashMap, ops::Range};

use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::Rect;

use super::{table::Table, table_geometry::ROW_HEIGHT};
use crate::renderer::text::shape_single_line;

pub(super) const CELL_FONT: f32 = 13.0;
pub(super) const HEADER_FONT: f32 = 16.0;
const HEADER_HEIGHT: f32 = 30.0;
const MINIMUM_COLUMN: f32 = 34.0;

pub(super) struct Columns {
    widths: Vec<f32>,
    gutter: f32,
}

pub(super) struct Grid {
    pub viewport: Rect,
    pub horizontal_header: Rect,
    pub vertical_header: Rect,
    pub corner: Rect,
    pub horizontal_scroll: Option<Rect>,
    pub vertical_scroll: Option<Rect>,
    pub content_size: (f32, f32),
    offsets: Vec<f32>,
}

/// 【文字识别】【内容测量】用 fonts、cache 测量 text 的实际字形与前进宽度，返回 Qt 单元格内容宽度，不截断多行值
fn content_width(fonts: &mut FontSystem, cache: &mut SwashCache, text: &str) -> f32 {
    let (buffer, _, _) = shape_single_line(
        fonts,
        text,
        CELL_FONT,
        cosmic_text::Weight::NORMAL,
        cosmic_text::Style::Normal,
    );
    let mut width: f32 = 0.0;
    for run in buffer.layout_runs() {
        let mut left: f32 = 0.0;
        // 1. 【文字识别】【固定精度】原版每个字形前进量保存为六十四分之一像素，长单元格必须先量化再累计
        let fixed = |advance: f32| (advance * 64.0).trunc() / 64.0;
        let mut right = (run.line_w
            + run
                .glyphs
                .iter()
                .map(|glyph| fixed(glyph.w) - glyph.w)
                .sum::<f32>())
        .ceil();
        let mut glyphs = run.glyphs.iter().collect::<Vec<_>>();
        glyphs.sort_by(|a, b| a.x.total_cmp(&b.x));
        let mut adjustment = 0.0;
        for glyph in glyphs {
            let advance = fixed(glyph.w);
            let (key, _, _) = cosmic_text::CacheKey::new(
                glyph.font_id,
                glyph.glyph_id,
                glyph.font_size,
                (0.0, 0.0),
                glyph.font_weight,
                glyph.cache_key_flags | cosmic_text::CacheKeyFlags::DISABLE_HINTING,
            );
            if let Some(commands) = cache.get_outline_commands(fonts, key) {
                let mut path = tiny_skia::PathBuilder::new();
                for command in commands {
                    match command {
                        cosmic_text::Command::MoveTo(p) => path.move_to(p.x, p.y),
                        cosmic_text::Command::LineTo(p) => path.line_to(p.x, p.y),
                        cosmic_text::Command::QuadTo(c, p) => path.quad_to(c.x, c.y, p.x, p.y),
                        cosmic_text::Command::CurveTo(a, b, p) => {
                            path.cubic_to(a.x, a.y, b.x, b.y, p.x, p.y)
                        }
                        cosmic_text::Command::Close => path.close(),
                    }
                }
                if let Some(bounds) = path.finish().and_then(|path| path.compute_tight_bounds()) {
                    let x = glyph.x + adjustment + glyph.font_size * glyph.x_offset;
                    left = left.min((x + bounds.left()).floor());
                    right = right.max((x + bounds.right()).ceil());
                    // 1. 【文字识别】【字形悬垂】原版按整数前进宽度计算负右边距，保留末字悬垂而非只取整行前进宽度
                    let right_bearing = advance.round() - bounds.right().ceil();
                    right = right.max((x + advance - right_bearing.min(0.0)).ceil());
                }
            }
            adjustment += advance - glyph.w;
        }
        width = width.max(right - left);
    }
    width + 7.0
}

impl Columns {
    /// 【文字识别】【初始列宽】用 table、fonts、headers 测量原始列宽和行号边栏，返回稳定尺寸；编辑内容不自动重算
    pub fn new(table: &Table, fonts: &mut FontSystem, headers: &mut FontSystem) -> Self {
        // 1. 【文字识别】【按需测量】相同文字只排版一次，初始尺寸不随窗口或编辑变化
        let mut cache = SwashCache::new();
        let mut measured = HashMap::new();
        let mut widths = vec![MINIMUM_COLUMN; table.columns()];
        for row in &table.rows {
            for (column, text) in row.iter().enumerate().take(widths.len()) {
                let width = *measured.entry(text.as_str()).or_insert_with(|| {
                    crate::fonts::ocr_ui::ensure_text(fonts, text);
                    content_width(fonts, &mut cache, text)
                });
                widths[column] = widths[column].max(width);
            }
        }
        // 2. 【文字识别】【行号尺寸】原版表头独立使用平台字体，粗体尺寸提示决定行号边栏宽度
        let digits = table.rows.len().max(1).to_string();
        let (_, width, _) = shape_single_line(
            headers,
            &digits,
            HEADER_FONT,
            cosmic_text::Weight::BOLD,
            cosmic_text::Style::Normal,
        );
        Self {
            widths,
            gutter: width.ceil() + 17.0,
        }
    }

    /// 【文字识别】【网格生成】按 size、rows 计算本地表头、滚动保留区与末列拉伸，返回与命中和绘制共用的布局
    pub fn grid(&self, size: (f32, f32), rows: usize) -> Grid {
        let content_height = rows as f32 * ROW_HEIGHT;
        let natural_width: f32 = self.widths.iter().sum();
        let mut vertical = false;
        let mut horizontal = false;
        // 1. 【文字识别】【滚动联动】两方向滚动条会压缩另一方向视区，按稳定状态重新判断
        for _ in 0..3 {
            let width = size.0 - 2.0 - self.gutter - if vertical { 6.0 } else { 0.0 };
            let height = size.1 - 2.0 - HEADER_HEIGHT - if horizontal { 14.0 } else { 0.0 };
            horizontal = natural_width > width;
            vertical = content_height > height;
        }
        let viewport = Rect::from_xywh(
            self.gutter + 1.0,
            HEADER_HEIGHT + 1.0,
            (size.0 - 2.0 - self.gutter - if vertical { 6.0 } else { 0.0 }).max(1.0),
            (size.1 - 2.0 - HEADER_HEIGHT - if horizontal { 14.0 } else { 0.0 }).max(1.0),
        )
        .unwrap();
        let mut widths = self.widths.clone();
        let preceding = widths
            .iter()
            .take(widths.len().saturating_sub(1))
            .sum::<f32>();
        if let Some(last) = widths.last_mut() {
            *last = last.max(viewport.width() - preceding);
        }
        let mut offsets = Vec::with_capacity(widths.len() + 1);
        offsets.push(0.0);
        for width in widths {
            offsets.push(offsets.last().copied().unwrap() + width);
        }
        Grid {
            viewport,
            horizontal_header: Rect::from_xywh(viewport.x(), 1.0, viewport.width(), HEADER_HEIGHT)
                .unwrap(),
            vertical_header: Rect::from_xywh(1.0, viewport.y(), self.gutter, viewport.height())
                .unwrap(),
            corner: Rect::from_xywh(1.0, 1.0, self.gutter, HEADER_HEIGHT).unwrap(),
            horizontal_scroll: horizontal.then(|| {
                Rect::from_xywh(
                    1.0,
                    size.1 - 15.0,
                    size.0 - 2.0 - if vertical { 6.0 } else { 0.0 },
                    14.0,
                )
                .unwrap()
            }),
            vertical_scroll: vertical.then(|| {
                Rect::from_xywh(
                    size.0 - 7.0,
                    1.0,
                    6.0,
                    size.1 - 2.0 - if horizontal { 14.0 } else { 0.0 },
                )
                .unwrap()
            }),
            content_size: (offsets.last().copied().unwrap_or(0.0), content_height),
            offsets,
        }
    }
}

impl Grid {
    /// 【文字识别】【列数读取】返回网格包含的真实列数，空表格返回零
    pub fn columns(&self) -> usize {
        self.offsets.len().saturating_sub(1)
    }

    /// 【文字识别】【列区域】按 column 返回正文内容起点与宽度，不含表头或滚动偏移
    pub fn column(&self, column: usize) -> (f32, f32) {
        (
            self.offsets[column],
            self.offsets[column + 1] - self.offsets[column],
        )
    }

    /// 【文字识别】【列定位】把 x 正文坐标转换为列号，左右越界返回空值
    pub fn column_at(&self, x: f32) -> Option<usize> {
        if x < 0.0 || x >= self.content_size.0 {
            return None;
        }
        Some(
            self.offsets
                .partition_point(|offset| *offset <= x)
                .saturating_sub(1),
        )
    }

    /// 【文字识别】【可见列】按 scroll 返回与正文视区相交的列范围，不遍历隐藏单元格
    pub fn visible_columns(&self, scroll: f32) -> Range<usize> {
        let count = self.offsets.len().saturating_sub(1);
        let first = self.column_at(scroll).unwrap_or(count);
        let last = self
            .offsets
            .partition_point(|offset| *offset < scroll + self.viewport.width())
            .min(count);
        first..last.max(first)
    }

    /// 【文字识别】【滚动边界】按 scroll 限制两方向内容偏移，返回不超出网格的坐标
    pub fn clamp_scroll(&self, scroll: (f32, f32)) -> (f32, f32) {
        (
            scroll
                .0
                .clamp(0.0, (self.content_size.0 - self.viewport.width()).max(0.0)),
            scroll
                .1
                .clamp(0.0, (self.content_size.1 - self.viewport.height()).max(0.0)),
        )
    }
}

#[cfg(test)]
mod tests;
