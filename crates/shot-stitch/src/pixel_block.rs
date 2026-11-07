//! 【长图拼接】【独立像素页】每块使用独立匿名映射，释放不受相邻堆对象影响

use memmap2::MmapMut;
use shot_image::ImageError;
use std::ops::{Deref, DerefMut};

pub(crate) struct PixelBlock(MmapMut);

impl PixelBlock {
    /// 【长图拼接】【像素页分配】按pixels创建清零的独立块，返回分配结果，不使用通用堆保存像素
    pub(crate) fn allocate(pixels: usize) -> Result<Self, ImageError> {
        let bytes = pixels
            .checked_mul(std::mem::size_of::<u32>())
            .filter(|bytes| *bytes > 0)
            .ok_or(ImageError::Dimensions)?;
        // 1. 【长图拼接】【映射所有权】映射库独占此内存，丢弃所有权时直接解除映射并释放常驻页
        let mapping = MmapMut::map_anon(bytes).map_err(|_| ImageError::Allocation)?;
        bytemuck::try_cast_slice::<u8, u32>(&mapping).map_err(|_| ImageError::Allocation)?;
        Ok(Self(mapping))
    }

    /// 【长图拼接】【块字节容量】无参数，返回实际映射字节长度，不含内核页对齐余量
    pub(crate) fn allocated_bytes(&self) -> usize {
        self.0.len()
    }
}

impl Deref for PixelBlock {
    type Target = [u32];

    /// 【长图拼接】【只读像素】无参数，返回初始化时已经校验对齐的ARGB像素切片，不复制字节
    fn deref(&self) -> &Self::Target {
        bytemuck::cast_slice(&self.0)
    }
}

impl DerefMut for PixelBlock {
    /// 【长图拼接】【可写像素】无参数，返回独占ARGB像素切片，调用者不能与只读切片同时持有
    fn deref_mut(&mut self) -> &mut Self::Target {
        bytemuck::cast_slice_mut(&mut self.0)
    }
}
