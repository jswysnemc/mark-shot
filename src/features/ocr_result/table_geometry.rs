//! 【文字识别】【表格行布局】有上限的动态行高与快速可见行定位

use std::ops::Range;

use super::table::Table;

pub(super) struct RowGeometry {
    heights: Vec<f32>,
    offsets: Vec<f32>,
}

/// 【文字识别】【行高计算】根据 values 中换行数计算行高；最多展示五行，避免单元格过大
fn row_height(values: &[String]) -> f32 {
    let lines = values
        .iter()
        .map(|value| value.split('\n').count())
        .max()
        .unwrap_or(1)
        .min(5);
    (lines as f32 * crate::theme::font::label() * 1.65 + 10.0).clamp(34.0, 128.0)
}

impl RowGeometry {
    /// 【文字识别】【行布局初始化】为 table 计算每行高度与累计坐标；返回不依赖窗口大小的行布局
    pub fn new(table: &Table) -> Self {
        let heights: Vec<_> = table.rows.iter().map(|row| row_height(row)).collect();
        let mut offsets = Vec::with_capacity(heights.len() + 1);
        offsets.push(0.0);
        for height in &heights {
            offsets.push(offsets.last().copied().unwrap_or(0.0) + height);
        }
        Self { heights, offsets }
    }

    /// 【文字识别】【行高更新】按 values 更新 row 的行高与后续坐标；返回高度是否改变
    pub fn update(&mut self, row: usize, values: &[String]) -> bool {
        let height = row_height(values);
        let difference = height - self.heights[row];
        if difference.abs() < 0.01 {
            return false;
        }
        self.heights[row] = height;
        for offset in &mut self.offsets[row + 1..] {
            *offset += difference;
        }
        true
    }

    /// 【文字识别】【行顶部】返回 row 在表格正文内的顶部坐标
    pub fn top(&self, row: usize) -> f32 {
        self.offsets[row]
    }

    /// 【文字识别】【行高度】返回 row 当前高度
    pub fn height(&self, row: usize) -> f32 {
        self.heights[row]
    }

    /// 【文字识别】【总行高度】返回所有行累计高度；空表格返回零
    pub fn total_height(&self) -> f32 {
        self.offsets.last().copied().unwrap_or(0.0)
    }

    /// 【文字识别】【坐标定位】将正文 y 坐标转换为行号；表格外坐标返回空值
    pub fn at(&self, y: f32) -> Option<usize> {
        if y < 0.0 || y >= self.total_height() {
            return None;
        }
        Some(
            self.offsets
                .partition_point(|offset| *offset <= y)
                .saturating_sub(1),
        )
    }

    /// 【文字识别】【可见行】按 scroll、height 返回与视区相交的行范围；不遍历不可见单元格
    pub fn visible(&self, scroll: f32, height: f32) -> Range<usize> {
        let start = self.at(scroll).unwrap_or(self.heights.len());
        let end = self
            .offsets
            .partition_point(|offset| *offset < scroll + height)
            .min(self.heights.len());
        start..end.max(start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【行布局测试】【多行定位】验证多行高度、后续行坐标与可见范围同步更新
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
        assert!(geometry.height(1) > geometry.height(0));
        assert_eq!(geometry.at(geometry.top(2) + 1.0), Some(2));
        assert_eq!(geometry.at(geometry.total_height()), None);
        assert_eq!(
            geometry.visible(geometry.top(1), geometry.height(1) + 1.0),
            1..3
        );
    }

    /// 【行布局测试】【高度恢复】验证编辑换行后更新后续坐标，删除换行后恢复紧凑高度
    #[test]
    fn editing_updates_following_rows_and_caps_large_cells() {
        let table = Table {
            rows: vec![vec!["one".into()], vec!["two".into()]],
        };
        let mut geometry = RowGeometry::new(&table);
        let original = geometry.top(1);
        assert!(geometry.update(0, &["a\nb\nc".into()]));
        assert!(geometry.top(1) > original);
        geometry.update(0, &["many\n".repeat(100)]);
        assert!(geometry.height(0) <= 128.0);
        assert!(geometry.update(0, &["single".into()]));
        assert_eq!(geometry.top(1), original);
    }
}
