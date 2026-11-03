//! 【文字识别】【表格推断】根据文字框位置重建矩形行列，不把普通段落当成表格

use crate::ocr::OcrLine;

const MAX_COLUMNS: usize = 64;
const MAX_ROWS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub rows: Vec<Vec<String>>,
}

impl Table {
    /// 【文字识别】【表格尺寸】返回表格列数；空表格返回零
    pub fn columns(&self) -> usize {
        self.rows.first().map_or(0, Vec::len)
    }

    /// 【文字识别】【单元格修改】将 row、column 对应内容替换为 text；越界返回 false
    pub fn set_cell(&mut self, row: usize, column: usize, text: String) -> bool {
        let Some(cell) = self.rows.get_mut(row).and_then(|row| row.get_mut(column)) else {
            return false;
        };
        *cell = text;
        true
    }
}

struct Row<'a> {
    cells: Vec<&'a OcrLine>,
    center: f32,
    height: f32,
}

/// 【文字识别】【行聚合】将 lines 按垂直中心聚成行；返回按位置排序的文字框
fn group_rows(lines: &[OcrLine]) -> Vec<Row<'_>> {
    let mut cells: Vec<_> = lines
        .iter()
        .filter(|line| !line.text.trim().is_empty())
        .collect();
    cells.sort_by(|a, b| {
        a.bounds
            .top()
            .total_cmp(&b.bounds.top())
            .then(a.bounds.left().total_cmp(&b.bounds.left()))
    });
    let mut rows: Vec<Row<'_>> = Vec::new();
    for cell in cells {
        let height = cell.bounds.height();
        let center = cell.bounds.top() + height / 2.0;
        let near = rows
            .iter_mut()
            .rev()
            .take(4)
            .find(|row| (row.center - center).abs() <= row.height.min(height) * 0.55);
        if let Some(row) = near {
            let count = row.cells.len() as f32;
            row.center = (row.center * count + center) / (count + 1.0);
            row.height = (row.height * count + height) / (count + 1.0);
            row.cells.push(cell);
        } else {
            rows.push(Row {
                cells: vec![cell],
                center,
                height,
            });
        }
    }
    rows.sort_by(|a, b| a.center.total_cmp(&b.center));
    for row in &mut rows {
        row.cells
            .sort_by(|a, b| a.bounds.left().total_cmp(&b.bounds.left()));
    }
    rows
}

/// 【文字识别】【列定位】以 seed 中相邻文字框之间的空隙确定列边界；返回 cell 对应列
fn column_for(seed: &[&OcrLine], cell: &OcrLine) -> usize {
    let center = cell.bounds.left() + cell.bounds.width() / 2.0;
    seed.windows(2)
        .position(|pair| center < (pair[0].bounds.right() + pair[1].bounds.left()) / 2.0)
        .unwrap_or(seed.len() - 1)
}

/// 【文字识别】【表格推断】从 lines 中推断至少两行、两列的表格；段落或稀疏布局返回空值
pub fn infer(lines: &[OcrLine]) -> Option<Table> {
    // 1. 聚合文字行，以单元格最多的一行作为列位置参考
    let rows = group_rows(lines);
    if rows.len() < 2 || rows.len() > MAX_ROWS {
        return None;
    }
    let seed = &rows.iter().max_by_key(|row| row.cells.len())?.cells;
    let columns = seed.len();
    if !(2..=MAX_COLUMNS).contains(&columns) {
        return None;
    }
    if seed
        .windows(2)
        .any(|pair| pair[0].bounds.right() > pair[1].bounds.left())
    {
        return None;
    }

    // 2. 根据列位置填入单元格，缺失文字保留为空格，不移动后续列
    let mut result = Vec::with_capacity(rows.len());
    let mut populated_rows = 0;
    let mut occupied = 0;
    for row in &rows {
        let mut cells = vec![String::new(); columns];
        for cell in &row.cells {
            let index = column_for(seed, cell);
            if !cells[index].is_empty() {
                cells[index].push(' ');
            }
            cells[index].push_str(&crate::ocr::bidi::to_logical(cell.text.trim()));
        }
        let count = cells.iter().filter(|cell| !cell.is_empty()).count();
        populated_rows += usize::from(count >= 2);
        occupied += count;
        result.push(cells);
    }

    // 3. 至少两行具有多列文字，并限制空单元格比例以排除普通段落
    if populated_rows < 2 || occupied * 2 < result.len() * columns {
        return None;
    }
    Some(Table { rows: result })
}
