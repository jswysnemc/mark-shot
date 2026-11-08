//! 【长图拼接】【所有权交接】逐块输出最终像素，已写入的原始块立即释放

use crate::{Axis, long_rows::LongRows};
use shot_image::ImageError;

pub struct StitchedImage {
    rows: LongRows,
    axis: Axis,
}

impl StitchedImage {
    /// 【长图拼接】【结果构造】按rows与axis持有完整原图，返回可消耗图像，不创建完整快照
    pub(crate) fn new(rows: LongRows, axis: Axis) -> Self {
        Self { rows, axis }
    }

    /// 【长图拼接】【显示宽度】无参数，返回最终图像宽度，横轴使用内部行数
    pub fn width(&self) -> i32 {
        match self.axis {
            Axis::Vertical => self.rows.width as i32,
            Axis::Horizontal => self.rows.height as i32,
        }
    }

    /// 【长图拼接】【显示高度】无参数，返回最终图像高度，横轴使用内部行宽
    pub fn height(&self) -> i32 {
        match self.axis {
            Axis::Vertical => self.rows.height as i32,
            Axis::Horizontal => self.rows.width as i32,
        }
    }

    /// 【长图拼接】【原图借用】无参数，返回当前拥有像素的只读视图，供逐行编码，不创建快照
    pub fn view(&self) -> crate::StitchedView<'_> {
        crate::StitchedView::new(&self.rows, self.axis)
    }

    /// 【长图拼接】【逐行消费】按显示顺序向 write 交付每行 ARGB，返回回调错误；纵向及时释放已交付块，横向仅分配一行转置缓冲
    pub fn for_each_row<Error>(
        self,
        mut write: impl FnMut(&[u32]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        match self.axis {
            Axis::Vertical => {
                let mut start = self.rows.start;
                let mut remaining = self.rows.height;
                for block in self.rows.blocks {
                    let count = (self.rows.block_rows - start).min(remaining);
                    for row in start..start + count {
                        write(&block[row * self.rows.width..(row + 1) * self.rows.width])?;
                    }
                    remaining -= count;
                    start = 0;
                }
            }
            Axis::Horizontal => {
                let mut row = vec![0; self.rows.height];
                for column in 0..self.rows.width {
                    for (index, pixel) in row.iter_mut().enumerate() {
                        *pixel = self.rows.row(index)[column];
                    }
                    write(&row)?;
                }
            }
        }
        Ok(())
    }

    /// 【长图拼接】【逐块转换】消耗原图并将全部像素写入target，输出不透明RGBA，返回尺寸结果，每块完成后释放其存储
    pub fn write_rgba(self, target: &mut [u8]) -> Result<(), ImageError> {
        let width = self.width() as usize;
        let height = self.height() as usize;
        if width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            != Some(target.len())
        {
            return Err(ImageError::BufferTooSmall);
        }
        let source_width = self.rows.width;
        let mut start = self.rows.start;
        let mut y = 0;
        // 1. 【长图拼接】【消耗原块】每次只持有当前块，输出页按写入位置生效，不保留已经交付的原始像素
        for block in self.rows.blocks {
            let count = (self.rows.block_rows - start).min(self.rows.height - y);
            for row in 0..count {
                let source = (start + row) * source_width;
                for (x, value) in block[source..source + source_width].iter().enumerate() {
                    let output = match self.axis {
                        Axis::Vertical => (y + row) * width + x,
                        Axis::Horizontal => x * width + y + row,
                    } * 4;
                    target[output..output + 4].copy_from_slice(&[
                        (value >> 16) as u8,
                        (value >> 8) as u8,
                        *value as u8,
                        255,
                    ]);
                }
            }
            y += count;
            start = 0;
        }
        Ok(())
    }
}
