//! 【长图拼接】【只读视图】借用内部行缓冲，横向输出只交换坐标，不分配完整转置图

use crate::Axis;
use crate::long_rows::LongRows;

#[derive(Clone, Copy)]
pub struct StitchedView<'a> {
    rows: &'a LongRows,
    axis: Axis,
}

impl<'a> StitchedView<'a> {
    /// 【长图拼接】【视图构造】按 rows 和 axis 保存只读借用，返回显示坐标视图，不复制像素
    pub(crate) fn new(rows: &'a LongRows, axis: Axis) -> Self {
        Self { rows, axis }
    }

    /// 【长图拼接】【显示宽度】无参数，返回原始输出宽度，横向读取内部行数
    pub fn width(self) -> i32 {
        match self.axis {
            Axis::Vertical => self.rows.width as i32,
            Axis::Horizontal => self.rows.height as i32,
        }
    }

    /// 【长图拼接】【显示高度】无参数，返回原始输出高度，横向读取内部行宽
    pub fn height(self) -> i32 {
        match self.axis {
            Axis::Vertical => self.rows.height as i32,
            Axis::Horizontal => self.rows.width as i32,
        }
    }

    /// 【长图拼接】【像素读取】按显示坐标 x、y 返回原始 ARGB 像素，越界调用视为调用方错误
    pub fn pixel(self, x: i32, y: i32) -> u32 {
        assert!(x >= 0 && x < self.width() && y >= 0 && y < self.height());
        match self.axis {
            Axis::Vertical => self.rows.row(y as usize)[x as usize],
            Axis::Horizontal => self.rows.row(x as usize)[y as usize],
        }
    }
}
