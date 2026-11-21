//! 【贴图】【表格绘制】测量单元格宽度、分配列宽并绘制带边框、表头与背景的富文本表格

use super::{
    Align, Attrs, Buffer, FontSystem, Palette, block_height, build_span_buffer, draw_decorations,
    draw_span_backgrounds, fill_rect, is_light, max_line_width,
};
use crate::renderer::Painter;
use crate::theme::Rgba;

const H_PAD: f32 = 8.0;
const V_PAD: f32 = 6.0;
const MIN_COL: f32 = 48.0;
const BORDER: f32 = 1.0;

/// 【贴图】【表格数据】记录预处理后的行与已解析片段属性
pub(super) struct Table {
    pub(super) rows: Vec<Row>,
}

/// 【贴图】【表格行数据】记录表头标记、行背景与单元格
pub(super) struct Row {
    pub(super) header: bool,
    pub(super) background: Option<Rgba>,
    pub(super) cells: Vec<Cell>,
}

/// 【贴图】【表格单元数据】记录表头标记、对齐、背景与带属性的片段
pub(super) struct Cell {
    pub(super) header: bool,
    pub(super) align: Align,
    pub(super) background: Option<Rgba>,
    pub(super) spans: Vec<(String, Attrs<'static>)>,
}

/// 【贴图】【已排表格】记录最终宽度、高度与逐行布局
pub(super) struct LaidTable {
    pub(super) width: f32,
    pub(super) height: f32,
    rows: Vec<LaidRow>,
}

/// 【贴图】【已排表格行】记录行高、背景、表头标记与单元
struct LaidRow {
    height: f32,
    background: Option<Rgba>,
    header: bool,
    cells: Vec<LaidCell>,
}

/// 【贴图】【已排表格单元】记录相对横坐标、宽度、文本缓冲、背景与表头标记
struct LaidCell {
    x: f32,
    width: f32,
    buffer: Buffer,
    background: Option<Rgba>,
    header: bool,
}

/// 【贴图】【表格宽度】按表格自然列宽返回含左右内边距的期望总宽
pub(super) fn intrinsic_width(fonts: &mut FontSystem, table: &Table) -> f32 {
    natural_widths(fonts, table).iter().sum()
}

/// 【贴图】【列宽分配】按自然列宽与可用宽度公平分配，窄列保留自然宽、宽列承担不足
fn column_widths(natural: &[f32], available: f32) -> Vec<f32> {
    let count = natural.len();
    if count == 0 {
        return Vec::new();
    }
    let widths: Vec<f32> = natural.iter().map(|width| width.max(MIN_COL)).collect();
    let total: f32 = widths.iter().sum();
    if total <= available {
        return widths;
    }
    let baseline = MIN_COL * count as f32;
    if baseline >= available {
        return vec![(available / count as f32).max(1.0); count];
    }
    let mut result = vec![0.0_f32; count];
    let mut order: Vec<usize> = (0..count).collect();
    order.sort_by(|a, b| widths[*a].total_cmp(&widths[*b]));
    let mut remaining = available;
    let mut remaining_cols = count;
    let mut index = 0;
    while index < order.len() {
        let column = order[index];
        let share = remaining / remaining_cols as f32;
        if widths[column] <= share {
            result[column] = widths[column];
            remaining -= widths[column];
            remaining_cols -= 1;
            index += 1;
        } else {
            break;
        }
    }
    let mut open: Vec<usize> = order[index..].to_vec();
    while !open.is_empty() {
        let sum: f32 = open.iter().map(|column| widths[*column]).sum();
        let mut too_small = Vec::new();
        for column in &open {
            if remaining * widths[*column] / sum < MIN_COL {
                too_small.push(*column);
            }
        }
        if too_small.is_empty() {
            for column in &open {
                result[*column] = remaining * widths[*column] / sum;
            }
            break;
        }
        for column in &too_small {
            result[*column] = MIN_COL;
            remaining -= MIN_COL;
        }
        open.retain(|column| !too_small.contains(column));
    }
    result
}

/// 【贴图】【内边距列宽】按单元格文本宽返回含左右内边距且不小于最小列宽的列宽
fn padded(width: f32) -> f32 {
    (width + H_PAD * 2.0).max(MIN_COL)
}

/// 【贴图】【自然列宽】按未换行单元格宽度返回每列含内边距的最宽列宽
fn natural_widths(fonts: &mut FontSystem, table: &Table) -> Vec<f32> {
    let columns = table
        .rows
        .iter()
        .map(|row| row.cells.len())
        .max()
        .unwrap_or(0);
    let mut widths = vec![0.0_f32; columns];
    for row in &table.rows {
        for (index, cell) in row.cells.iter().enumerate() {
            let buffer = build_span_buffer(fonts, &cell.spans, cell.align, None);
            widths[index] = widths[index].max(max_line_width(&buffer));
        }
    }
    widths.into_iter().map(padded).collect()
}

/// 【贴图】【表格排版】按可用宽度分配列宽、换行单元格并累计行高，返回已排表格
pub(super) fn layout(fonts: &mut FontSystem, table: &Table, available: f32) -> LaidTable {
    let columns = column_widths(&natural_widths(fonts, table), available);
    let mut rows = Vec::with_capacity(table.rows.len());
    let mut height = 0.0_f32;
    for row in &table.rows {
        let mut cells = Vec::with_capacity(columns.len());
        let mut x = 0.0_f32;
        let mut row_height = 0.0_f32;
        for (index, width) in columns.iter().copied().enumerate() {
            let source = row.cells.get(index);
            let inner = (width - H_PAD * 2.0).max(1.0);
            let empty: &[(String, Attrs<'static>)] = &[];
            let spans = source.map_or(empty, |cell| cell.spans.as_slice());
            let align = source.map_or(Align::Left, |cell| cell.align);
            let buffer = build_span_buffer(fonts, spans, align, Some(inner));
            row_height = row_height.max(block_height(&buffer));
            cells.push(LaidCell {
                x,
                width,
                buffer,
                background: source.and_then(|cell| cell.background),
                header: source.is_some_and(|cell| cell.header),
            });
            x += width;
        }
        let row_height = row_height + V_PAD * 2.0;
        rows.push(LaidRow {
            height: row_height,
            background: row.background,
            header: row.header,
            cells,
        });
        height += row_height;
    }
    LaidTable {
        width: columns.iter().sum(),
        height,
        rows,
    }
}

/// 【贴图】【表格绘制】按已排表格绘制行/单元背景、边框、文字、片段底色与装饰线
pub(super) fn draw(
    painter: &mut Painter,
    table: &LaidTable,
    origin: (f32, f32),
    palette: &Palette,
    backgrounds: &[Option<Rgba>],
) {
    let border = crate::theme::color::border().color();
    let left = origin.0.round();
    let top = origin.1.round();
    let right = (origin.0 + table.width).round();
    let width = (right - left).max(BORDER);
    let mut y = origin.1;
    let mut row_top = top;
    for row in &table.rows {
        let next_y = y + row.height;
        let row_bottom = next_y.round();
        let row_height = (row_bottom - row_top).max(BORDER);
        if let Some(background) = row.background {
            fill_rect(
                painter,
                left,
                row_top,
                width,
                row_height,
                background.color(),
            );
        }
        for cell in &row.cells {
            let cell_left = (origin.0 + cell.x).round();
            let cell_right = (origin.0 + cell.x + cell.width).round();
            let cell_width = (cell_right - cell_left).max(BORDER);
            let background = cell.background.or_else(|| {
                ((cell.header || row.header) && is_light(palette.canvas))
                    .then(crate::theme::color::field)
            });
            if let Some(background) = background {
                fill_rect(
                    painter,
                    cell_left,
                    row_top,
                    cell_width,
                    row_height,
                    background.color(),
                );
            }
            let text_origin = (origin.0 + cell.x + H_PAD, y + V_PAD);
            draw_span_backgrounds(painter, &cell.buffer, text_origin, backgrounds);
            painter.text_buffer(&cell.buffer, text_origin, palette.foreground.color());
            draw_decorations(
                painter,
                &cell.buffer,
                text_origin,
                palette.foreground.cosmic(),
            );
        }
        fill_rect(painter, left, row_bottom - BORDER, width, BORDER, border);
        y = next_y;
        row_top = row_bottom;
    }
    let bottom = row_top;
    let height = (bottom - top).max(BORDER);
    if let Some(first) = table.rows.first() {
        for cell in &first.cells {
            let cell_left = (origin.0 + cell.x).round();
            fill_rect(painter, cell_left, top, BORDER, height, border);
        }
        fill_rect(painter, right - BORDER, top, BORDER, height, border);
    }
    fill_rect(painter, left, top, width, BORDER, border);
}

#[cfg(test)]
mod tests {
    use super::column_widths;

    /// 【贴图测试】【窄列保留】无参数，窄列保持自然宽度而宽列吸收不足并合计可用宽度
    #[test]
    fn narrow_columns_keep_their_natural_width() {
        let widths = column_widths(&[146.0, 700.0], 720.0);
        assert_eq!(widths.len(), 2);
        assert!((widths[0] - 146.0).abs() < 0.01);
        assert!((widths[1] - 574.0).abs() < 0.01);
        assert!((widths.iter().sum::<f32>() - 720.0).abs() < 0.01);
    }

    /// 【贴图测试】【自然宽度】无参数，总宽不超可用宽度时原样返回自然列宽
    #[test]
    fn natural_widths_are_kept_when_they_fit() {
        assert_eq!(column_widths(&[146.0, 200.0], 720.0), vec![146.0, 200.0]);
    }

    /// 【贴图测试】【多窄列】无参数，多个窄列先被固定，剩余宽度由最宽列承担
    #[test]
    fn narrow_columns_are_fixed_before_wide_ones_shrink() {
        let widths = column_widths(&[60.0, 60.0, 700.0], 720.0);
        assert!((widths[0] - 60.0).abs() < 0.01);
        assert!((widths[1] - 60.0).abs() < 0.01);
        assert!((widths[2] - 600.0).abs() < 0.01);
        assert!((widths.iter().sum::<f32>() - 720.0).abs() < 0.01);
    }
}
