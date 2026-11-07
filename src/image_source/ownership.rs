//! 【图像原件】【像素所有权】只在真实修改或完整交付时转为独立内存，后台只读来源保持有效

use super::{Backing, ImageSource};
use tiny_skia::{Color, IntRect, Pixmap, PremultipliedColorU8};

impl ImageSource {
    /// 【图像原件】【独立交接】消耗来源并返回完整可修改原件，独占内存不复制，缓存按行读取后释放文件
    pub fn into_pixmap(self) -> Pixmap {
        match self.backing {
            Backing::Memory(pixels) => pixels,
            Backing::File(_) => {
                let full = IntRect::from_xywh(0, 0, self.width(), self.height()).unwrap();
                self.clone_rect(full)
                    .expect("cannot allocate original pixels")
            }
        }
    }

    /// 【图像原件】【修改入口】无参数，返回独立可修改像素，第一次修改缓存时按行读回，不修改只读文件
    pub fn pixmap_mut(&mut self) -> &mut Pixmap {
        if matches!(self.backing, Backing::File(_)) {
            let full = IntRect::from_xywh(0, 0, self.width(), self.height()).unwrap();
            let pixels = self
                .clone_rect(full)
                .expect("cannot allocate editable pixels");
            self.backing = Backing::Memory(pixels);
        }
        let Backing::Memory(pixels) = &mut self.backing else {
            unreachable!();
        };
        pixels
    }

    /// 【图像原件】【字节修改】无参数，返回独立预乘RGBA字节，缓存原件只在真实修改时转为内存
    pub fn data_mut(&mut self) -> &mut [u8] {
        self.pixmap_mut().data_mut()
    }

    /// 【图像原件】【像素修改】无参数，返回独立预乘像素切片，不修改已有只读文件
    pub fn pixels_mut(&mut self) -> &mut [PremultipliedColorU8] {
        self.pixmap_mut().pixels_mut()
    }

    /// 【图像原件】【原件填充】按color修改独立原件，返回值为空，其他共享来源由上层隔离
    pub fn fill(&mut self, color: Color) {
        self.pixmap_mut().fill(color);
    }
}

impl Clone for ImageSource {
    /// 【图像原件】【修改隔离】无参数，返回完整独立内存原件，只用于写时复制，不复制或修改既有文件
    fn clone(&self) -> Self {
        let full = IntRect::from_xywh(0, 0, self.width(), self.height()).unwrap();
        Self::memory(self.clone_rect(full).expect("cannot clone original pixels"))
    }
}
