//! 【文字识别】【表格行布局】原版固定行高及常数时间可见行定位

use std::ops::Range;

use super::table::Table;

pub(super) struct RowGeometry {
    count: usize,
}

pub(super) const ROW_HEIGHT: f32 = 28.0;

impl RowGeometry {
    /// 【文字识别】【行布局初始化】为 table 保存行数，返回原版固定行高布局，不为每一行分配坐标
    pub fn new(table: &Table) -> Self {
        Self {
            count: table.rows.len(),
        }
    }

    /// 【文字识别】【行顶部】返回 row 在表格正文内的顶部坐标
    pub fn top(&self, row: usize) -> f32 {
        row as f32 * ROW_HEIGHT
    }

    /// 【文字识别】【行高度】返回 row 当前高度
    pub fn height(&self, _row: usize) -> f32 {
        ROW_HEIGHT
    }

    /// 【文字识别】【总行高度】返回所有行累计高度；空表格返回零
    pub fn total_height(&self) -> f32 {
        self.count as f32 * ROW_HEIGHT
    }

    /// 【文字识别】【坐标定位】将正文 y 坐标转换为行号；表格外坐标返回空值
    pub fn at(&self, y: f32) -> Option<usize> {
        if y < 0.0 || y >= self.total_height() {
            return None;
        }
        Some((y / ROW_HEIGHT).floor() as usize)
    }

    /// 【文字识别】【可见行】按 scroll、height 返回与视区相交的行范围；不遍历不可见单元格
    pub fn visible(&self, scroll: f32, height: f32) -> Range<usize> {
        let start = ((scroll.max(0.0) / ROW_HEIGHT).floor() as usize).min(self.count);
        let end = (((scroll + height).max(0.0) / ROW_HEIGHT).ceil() as usize).min(self.count);
        start..end.max(start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【行布局测试】【原版固定高度】对照真实 Qt 表格初始与编辑状态，所有行保持原版固定高度，无参数、无返回值
    #[test]
    fn row_height_matches_actual_qt_before_and_after_multiline_edits() {
        let reference: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/qt-ocr-table-grid.json"
        ))
        .unwrap();
        for case in reference["cases"].as_array().unwrap() {
            for state in case["states"].as_array().unwrap() {
                let cells = state["state"]["cells"].as_array().unwrap();
                let rows = (0..3)
                    .map(|row| {
                        cells
                            .iter()
                            .filter(|cell| cell["row"] == row)
                            .map(|cell| cell["text"].as_str().unwrap().to_owned())
                            .collect()
                    })
                    .collect();
                let geometry = RowGeometry::new(&Table { rows });
                assert_eq!(
                    geometry.height(1),
                    state["state"]["rows"][1]["height"].as_f64().unwrap() as f32,
                    "{} {} {}",
                    case["theme"],
                    case["kind"],
                    state["name"]
                );
            }
        }
    }

    /// 【行布局测试】【多行定位】验证多行值不扩张行高，坐标与可见范围沿用原版固定高度
    #[test]
    fn multiline_rows_keep_hit_testing_and_visibility_aligned() {
        let table = Table {
            rows: vec![
                vec!["first".into()],
                vec!["a\nb".into()],
                vec!["last".into()],
            ],
        };
        let geometry = RowGeometry::new(&table);
        assert_eq!(geometry.height(1), geometry.height(0));
        assert_eq!(geometry.at(geometry.top(2) + 1.0), Some(2));
        assert_eq!(geometry.at(geometry.total_height()), None);
        assert_eq!(
            geometry.visible(geometry.top(1), geometry.height(1) + 1.0),
            1..3
        );
    }

    /// 【行布局测试】【空表格边界】验证空表格、负坐标和越界滚动不产生伪造行，无参数、无返回值
    #[test]
    fn empty_tables_and_outside_offsets_have_no_visible_rows() {
        let geometry = RowGeometry::new(&Table { rows: vec![] });
        assert_eq!(geometry.total_height(), 0.0);
        assert_eq!(geometry.at(-1.0), None);
        assert_eq!(geometry.at(0.0), None);
        assert_eq!(geometry.visible(0.0, 100.0), 0..0);
        let populated = RowGeometry::new(&Table {
            rows: vec![vec!["one".into()]],
        });
        assert_eq!(populated.visible(100.0, 40.0), 1..1);
    }
}
