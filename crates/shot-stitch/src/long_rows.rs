//! 【长图拼接】【分块行缓冲】固定块管理双向增长，只分配新块，不替换已有完整长图

use crate::pixel_block::PixelBlock;
use shot_image::{Image, ImageError};
use std::collections::VecDeque;

const BLOCK_ROWS: usize = 128;
const BLOCK_PIXELS: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PixelStorage {
    pub used_bytes: usize,
    pub allocated_bytes: usize,
    pub largest_allocation_bytes: usize,
    pub blocks: usize,
}

#[derive(Default)]
pub(crate) struct LongRows {
    pub(crate) blocks: VecDeque<PixelBlock>,
    pub(crate) width: usize,
    pub(crate) block_rows: usize,
    pub(crate) start: usize,
    pub(crate) height: usize,
}

impl LongRows {
    /// 【长图拼接】【行缓冲初始化】按image复制有效行到固定块，返回独立行缓冲，失败不修改旧长图
    pub(crate) fn from_image(image: &Image) -> Result<Self, ImageError> {
        let width = image.width() as usize;
        let mut rows = Self {
            width,
            block_rows: (BLOCK_PIXELS / width).clamp(1, BLOCK_ROWS),
            ..Self::default()
        };
        rows.reserve(0, image.height() as usize)?;
        rows.height = image.height() as usize;
        for y in 0..rows.height {
            rows.row_mut(y).copy_from_slice(image.row(y as i32));
        }
        Ok(rows)
    }

    /// 【长图拼接】【行读取】按y返回有效行，越界表示调用错误，不复制像素
    pub(crate) fn row(&self, y: usize) -> &[u32] {
        assert!(y < self.height);
        let row = self.start + y;
        let start = row % self.block_rows * self.width;
        &self.blocks[row / self.block_rows][start..start + self.width]
    }

    /// 【长图拼接】【行写入】按y返回有效行的可写切片，越界表示调用错误
    pub(crate) fn row_mut(&mut self, y: usize) -> &mut [u32] {
        assert!(y < self.height);
        let row = self.start + y;
        let start = row % self.block_rows * self.width;
        &mut self.blocks[row / self.block_rows][start..start + self.width]
    }

    /// 【长图拼接】【双端预留】按top与bottom预留新增行，返回分配结果，所有新增块完成后才改变原缓冲
    pub(crate) fn reserve(&mut self, top: usize, bottom: usize) -> Result<(), ImageError> {
        let front = top.saturating_sub(self.start).div_ceil(self.block_rows);
        let available = self.blocks.len() * self.block_rows;
        let back = (self.start + self.height + bottom)
            .saturating_sub(available)
            .div_ceil(self.block_rows);
        let count = front + back;
        if count == 0 {
            return Ok(());
        }
        // 1. 【长图拼接】【分配事务】先保留块索引和新像素，分配失败不能移动原来的行或释放其像素
        self.blocks
            .try_reserve(count)
            .map_err(|_| ImageError::Allocation)?;
        let mut added = VecDeque::new();
        added
            .try_reserve(count)
            .map_err(|_| ImageError::Allocation)?;
        for _ in 0..count {
            added.push_back(PixelBlock::allocate(self.width * self.block_rows)?);
        }
        // 2. 【长图拼接】【块交接】只移动块的所有权，不复制任何已有长图像素
        for _ in 0..front {
            self.blocks.push_front(added.pop_front().unwrap());
        }
        self.start += front * self.block_rows;
        self.blocks.extend(added);
        Ok(())
    }

    /// 【长图拼接】【空块回收】无参数，释放有效行之前或之后的完整空块，不移动有效像素，无返回值
    pub(crate) fn compact(&mut self) {
        while self.start >= self.block_rows {
            self.blocks.pop_front();
            self.start -= self.block_rows;
        }
        let needed = (self.start + self.height).div_ceil(self.block_rows);
        self.blocks.truncate(needed);
    }

    /// 【长图拼接】【分配统计】无参数，返回有效像素、当前块容量、最大块容量和块数，不包含帧与签名
    pub(crate) fn storage(&self) -> PixelStorage {
        let bytes = std::mem::size_of::<u32>();
        PixelStorage {
            used_bytes: self.height * self.width * bytes,
            allocated_bytes: self.blocks.iter().map(PixelBlock::allocated_bytes).sum(),
            largest_allocation_bytes: self
                .blocks
                .iter()
                .map(PixelBlock::allocated_bytes)
                .max()
                .unwrap_or(0),
            blocks: self.blocks.len(),
        }
    }
}
