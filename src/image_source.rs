//! 【图像原件】【只读像素】统一原件所有权与像素视图，绘制和裁切不改变原始颜色

mod cache_file;
mod cache_write;
mod ownership;
mod preview;
mod region_copy;
mod row_access;

use std::path::Path;
use tiny_skia::{IntRect, IntSize, Pixmap, PixmapRef};

const MEMORY_BYTES: usize = 4 * 1024 * 1024;

enum Backing {
    Memory(Pixmap),
    File(cache_file::CacheFile),
}

pub trait PixelView {
    /// 【图像原件】【像素借用】无参数，返回当前所有者的预乘RGBA视图，借用期间原始字节保持有效
    fn pixels_view(&self) -> PixmapRef<'_>;
}

pub struct ImageSource {
    backing: Backing,
    size: IntSize,
}

impl ImageSource {
    /// 【图像原件】【内存原件】按pixels保留只读原件，返回拥有原像素的来源，不复制缓冲
    pub fn memory(pixels: Pixmap) -> Self {
        let size = IntSize::from_wh(pixels.width(), pixels.height()).unwrap();
        Self {
            backing: Backing::Memory(pixels),
            size,
        }
    }

    /// 【图像原件】【缓存选择】按pixels返回原件，小图保留内存，大图使用系统缓存目录；缓存失败仍保留原像素
    pub fn store(pixels: Pixmap) -> Self {
        Self::store_kind(pixels, "image-source")
    }

    /// 【冻结截图】【交互原件】按pixels返回共享内存来源，复用唯一截图缓冲，避免每次重绘重新读取磁盘；滚动开始时由会话释放
    pub fn store_snapshot(pixels: Pixmap) -> Self {
        Self::memory(pixels)
    }

    /// 【图像原件】【来源目录】按pixels和kind选择私有磁盘目录，返回完整来源，失败保留原始像素
    fn store_kind(pixels: Pixmap, kind: &str) -> Self {
        if pixels.data().len() <= MEMORY_BYTES {
            return Self::memory(pixels);
        }
        let Some(directory) = dirs::cache_dir() else {
            log::error!("【图像原件】【缓存选择】没有可用缓存目录，继续使用原始像素");
            return Self::memory(pixels);
        };
        Self::store_in(pixels, &directory.join("mark-shot").join(kind))
    }

    /// 【图像原件】【缓存入口】按pixels和directory创建私有大图缓存，返回来源；文件错误不丢弃原像素
    pub(crate) fn store_in(pixels: Pixmap, directory: &Path) -> Self {
        if pixels.data().len() <= MEMORY_BYTES {
            return Self::memory(pixels);
        }
        let size = IntSize::from_wh(pixels.width(), pixels.height()).unwrap();
        match cache_file::CacheFile::new(pixels.as_ref(), directory) {
            Ok(file) => {
                log::info!(
                    "【图像原件】【缓存存储】原件={}x{}，字节数={}",
                    size.width(),
                    size.height(),
                    file.data().len()
                );
                Self {
                    backing: Backing::File(file),
                    size,
                }
            }
            Err(error) => {
                log::error!("【图像原件】【缓存存储】{error}，继续使用原始像素");
                Self::memory(pixels)
            }
        }
    }

    /// 【图像原件】【完整视图】无参数，返回预乘RGBA只读视图，不复制或缩放像素
    pub fn as_ref(&self) -> PixmapRef<'_> {
        PixmapRef::from_bytes(self.data(), self.width(), self.height()).unwrap()
    }

    /// 【图像原件】【像素宽度】无参数，返回原始设备像素宽度
    pub fn width(&self) -> u32 {
        self.size.width()
    }

    /// 【图像原件】【像素高度】无参数，返回原始设备像素高度
    pub fn height(&self) -> u32 {
        self.size.height()
    }

    /// 【图像原件】【完整尺寸】无参数，返回原始设备像素尺寸，不按逻辑比例取整
    pub fn size(&self) -> IntSize {
        self.size
    }

    /// 【图像原件】【单点读取】按x、y返回预乘原始像素，越界返回空值，不分配中间画布
    pub fn pixel(&self, x: u32, y: u32) -> Option<tiny_skia::PremultipliedColorU8> {
        self.as_ref().pixel(x, y)
    }

    /// 【图像原件】【像素切片】无参数，返回完整只读预乘像素切片，不改变透明度
    pub fn pixels(&self) -> &[tiny_skia::PremultipliedColorU8] {
        self.as_ref().pixels()
    }

    /// 【图像原件】【原始字节】无参数，返回原始预乘RGBA字节，不改变透明度
    pub fn data(&self) -> &[u8] {
        match &self.backing {
            Backing::Memory(pixels) => pixels.data(),
            Backing::File(file) => file.data(),
        }
    }

    /// 【图像原件】【区域行借用】按region逐行调用visit，行号为原始全局像素坐标，字节为该行相交RGBA范围，分批结束后归还读取页，无返回值
    #[cfg(target_os = "linux")]
    pub(crate) fn visit_rows(&self, region: IntRect, visit: impl FnMut(u32, &[u8])) {
        row_access::visit(self, region, visit);
    }

    /// 【图像原件】【有界行借用】按region和budget逐行调用visit，无返回值；保持完整原始行，每批结束归还读取页，最小批次为一行
    pub(crate) fn visit_rows_with_budget(
        &self,
        region: IntRect,
        budget: usize,
        visit: impl FnMut(u32, &[u8]),
    ) {
        row_access::visit_bounded(self, region, budget, visit);
    }

    /// 【图像原件】【裁切复制】按region返回与原件相交的完整像素，不进行透明混合
    pub fn clone_rect(&self, region: IntRect) -> Option<Pixmap> {
        region_copy::copy(self, region)
    }

    /// 【图像原件】【分批预览】按target、paint和transform绘制完整原件坐标，分批回收只读页，无返回值
    pub fn draw_preview(
        &self,
        target: &mut Pixmap,
        paint: &tiny_skia::PixmapPaint,
        transform: tiny_skia::Transform,
    ) {
        preview::draw(self, target, paint, transform, None);
    }

    /// 【图像原件】【损坏合成】按target、paint和region在完整原始像素坐标中分批绘制相交范围，不改变区域外像素，无返回值
    pub fn draw_region(
        &self,
        target: &mut Pixmap,
        paint: &tiny_skia::PixmapPaint,
        region: IntRect,
    ) {
        preview::draw(
            self,
            target,
            paint,
            tiny_skia::Transform::identity(),
            Some(region),
        );
    }

    /// 【图像原件】【读取页释放】无参数，结束本次只读访问后回收可重读页，无返回值，原始字节保持有效
    pub fn release_pages(&self) {
        if let Backing::File(file) = &self.backing {
            file.release_pages();
        }
    }

    /// 【图像原件】【文件字节】无参数，返回缓存文件大小，内存来源返回零；该数值不代表常驻内存
    pub fn cached_bytes(&self) -> usize {
        match &self.backing {
            Backing::Memory(_) => 0,
            Backing::File(file) => file.data().len(),
        }
    }

    /// 【图像原件】【区域版本】将pixels按position写入原件，文件来源创建独立版本，返回值为空；文件错误保留可修改的完整像素
    pub fn replace_pixels(&mut self, pixels: &Pixmap, position: (i32, i32)) {
        if region_copy::patch_bounds(self.size, pixels.as_ref(), position).is_none() {
            return;
        }
        if let Backing::Memory(target) = &mut self.backing {
            region_copy::patch_memory(target, pixels.as_ref(), position);
            return;
        }
        let Backing::File(file) = &self.backing else {
            unreachable!();
        };
        match file.replaced(self.size, pixels.as_ref(), position) {
            Ok(next) => self.backing = Backing::File(next),
            Err(error) => {
                log::error!("【图像原件】【区域版本】{error}，继续使用独立像素");
                let full = IntRect::from_xywh(0, 0, self.width(), self.height()).unwrap();
                if let Some(mut target) = self.clone_rect(full) {
                    region_copy::patch_memory(&mut target, pixels.as_ref(), position);
                    self.backing = Backing::Memory(target);
                } else {
                    log::error!("【图像原件】【区域版本】无法分配原件修改缓冲");
                }
            }
        }
    }
}

impl PixelView for ImageSource {
    /// 【图像原件】【缓存视图】无参数，借用来源中的完整预乘像素，不分配中间画布
    fn pixels_view(&self) -> PixmapRef<'_> {
        self.as_ref()
    }
}

impl PixelView for Pixmap {
    /// 【图像原件】【内存视图】无参数，借用现有画布的完整预乘像素，不分配中间画布
    fn pixels_view(&self) -> PixmapRef<'_> {
        self.as_ref()
    }
}

impl PixelView for crate::types::frozen_pixmap::FrozenPixmap {
    /// 【图像原件】【冻结视图】无参数，借用共享冻结截图的预乘像素，不复制分析输入
    fn pixels_view(&self) -> PixmapRef<'_> {
        self.as_ref()
    }
}

#[cfg(test)]
mod tests;
