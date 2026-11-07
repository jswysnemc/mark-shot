use crate::long_rows::LongRows;
use crate::profile::Signature;
use shot_image::buffer::allocate;
use shot_image::{Image, ImageError, MAX_PIXELS};

#[derive(Default)]
pub(crate) struct LongImage {
    pixels: LongRows,
    width: i32,
    pub cols: Signature,
    pub revision: u64,
}

impl LongImage {
    /// 【长图拼接】【种子重建】按seed和cols建立独立分块原图，成功后替换当前内容，返回分配结果
    pub fn reset(&mut self, seed: &Image, cols: Signature) -> Result<(), ImageError> {
        let pixels = LongRows::from_image(seed)?;
        *self = Self {
            pixels,
            width: seed.width(),
            cols,
            revision: self.revision.wrapping_add(1),
        };
        Ok(())
    }

    /// 【长图拼接】【内部宽度】无参数，返回内部行的像素宽度
    pub fn width(&self) -> i32 {
        self.width
    }

    /// 【长图拼接】【内部高度】无参数，返回有效行数，不包含两端空余行
    pub fn height(&self) -> i32 {
        self.pixels.height as i32
    }

    /// 【长图拼接】【分配统计】无参数，返回当前长图的有效字节、分配字节和最大单次像素分配，不包含帧与签名
    pub(crate) fn storage(&self) -> crate::PixelStorage {
        self.pixels.storage()
    }

    /// 【长图拼接】【行读取】按y返回分块中的有效像素行，越界表示调用错误
    pub fn row(&self, y: i32) -> &[u32] {
        assert!(y >= 0);
        self.pixels.row(y as usize)
    }

    /// 【长图拼接】【显式快照】无参数，复制有效行到兼容Image，返回独立完整快照或分配错误
    pub fn image(&self) -> Result<Image, ImageError> {
        let mut data = allocate(self.pixels.height * self.width as usize)?;
        for y in 0..self.pixels.height {
            let start = y * self.width as usize;
            data[start..start + self.width as usize].copy_from_slice(self.pixels.row(y));
        }
        Image::from_pixels(self.width, self.height(), data)
    }

    /// 【长图拼接】【连续借用】无参数，返回有效行的只读视图；空图返回 None，不复制预留区或原始像素
    pub fn view(&self) -> Option<&LongRows> {
        if self.pixels.height == 0 {
            return None;
        }
        Some(&self.pixels)
    }

    /// 【长图拼接】【行所有权】消耗当前长图并返回有效行块，签名随当前容器释放，不复制原始像素
    pub(crate) fn into_rows(self) -> LongRows {
        self.pixels
    }

    /// 【长图拼接】【增长空间】按 grow_top、grow_bottom 预留两端行数，返回容量结果；分配失败保留原始像素
    fn ensure_room(&mut self, grow_top: usize, grow_bottom: usize) -> Result<(), ImageError> {
        let width = self.width as usize;
        let target = self
            .pixels
            .height
            .checked_add(grow_top)
            .and_then(|n| n.checked_add(grow_bottom))
            .ok_or(ImageError::Dimensions)?;
        let limit = MAX_PIXELS / (width + crate::profile::BAND_COUNT);
        if target > limit {
            return Err(ImageError::Dimensions);
        }
        self.pixels.reserve(grow_top, grow_bottom)
    }

    /// 【长图拼接】【末端增长】按frame和cols追加rows行并替换trim行，返回更新结果，旧块不会参与整图搬移
    pub fn append(
        &mut self,
        frame: &Image,
        cols: &Signature,
        rows: i32,
        trim: i32,
    ) -> Result<(), ImageError> {
        let rows = rows.clamp(0, frame.height().min(cols.len() as i32)) as usize;
        let trim = trim.clamp(0, (self.height() - 1).max(0)) as usize;
        self.cols
            .try_reserve(rows)
            .map_err(|_| ImageError::Allocation)?;
        self.ensure_room(0, rows.saturating_sub(trim))?;
        let start = self.pixels.height - trim;
        self.cols.truncate(start);
        self.pixels.height = start + rows;
        for i in 0..rows {
            let y = frame.height() as usize - rows + i;
            self.pixels
                .row_mut(start + i)
                .copy_from_slice(frame.row(y as i32));
            self.cols.push(cols[y]);
        }
        self.pixels.compact();
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }

    /// 【长图拼接】【起端增长】按frame和cols前插rows行并替换trim行，返回更新结果，只分配新块
    pub fn prepend(
        &mut self,
        frame: &Image,
        cols: &Signature,
        rows: i32,
        trim: i32,
    ) -> Result<(), ImageError> {
        let rows = rows.clamp(0, frame.height().min(cols.len() as i32)) as usize;
        let trim = trim.clamp(0, (self.height() - 1).max(0)) as usize;
        let mut merged = Vec::new();
        merged
            .try_reserve_exact(rows + self.pixels.height - trim)
            .map_err(|_| ImageError::Allocation)?;
        merged.extend_from_slice(&cols[..rows]);
        merged.extend_from_slice(&self.cols[trim..]);
        self.ensure_room(rows.saturating_sub(trim), 0)?;
        self.pixels.start = self.pixels.start + trim - rows;
        self.pixels.height = self.pixels.height - trim + rows;
        for i in 0..rows {
            self.pixels.row_mut(i).copy_from_slice(frame.row(i as i32));
        }
        self.pixels.compact();
        self.cols = merged;
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
}
