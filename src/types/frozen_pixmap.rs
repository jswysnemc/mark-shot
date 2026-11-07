//! 【冻结截图】【共享像素】分析任务借用原始像素，修改时复制，普通读取不改变原图

use crate::image_source::ImageSource;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tiny_skia::Pixmap;

#[derive(Clone)]
pub struct FrozenPixmap(Arc<ImageSource>);

impl FrozenPixmap {
    /// 【冻结截图】【分析共享】无参数，返回原始像素的共享所有权，不复制截图字节
    pub fn share(&self) -> Arc<ImageSource> {
        self.0.clone()
    }

    /// 【冻结截图】【像素交接】消耗本容器并返回可修改像素，其他所有者仍存在时保持其原图
    pub fn into_pixmap(self) -> Pixmap {
        Arc::unwrap_or_clone(self.0).into_pixmap()
    }

    /// 【冻结截图】【私有缓存】按pixels返回共享只读来源，大图进入私有磁盘缓存，缓存失败保留原始像素
    pub fn cached(pixels: Pixmap) -> Self {
        Self(Arc::new(ImageSource::store_snapshot(pixels)))
    }
}

impl From<Pixmap> for FrozenPixmap {
    /// 【冻结截图】【容器构造】按 pixmap 保存共享像素，返回容器；不重新分配像素缓冲
    fn from(pixmap: Pixmap) -> Self {
        Self(Arc::new(ImageSource::memory(pixmap)))
    }
}

impl Deref for FrozenPixmap {
    type Target = ImageSource;

    /// 【冻结截图】【只读访问】无参数，返回原始像素引用
    fn deref(&self) -> &ImageSource {
        &self.0
    }
}

impl DerefMut for FrozenPixmap {
    /// 【冻结截图】【修改隔离】无参数，返回独立可修改像素；后台仍借用时复制，不污染其输入
    fn deref_mut(&mut self) -> &mut ImageSource {
        Arc::make_mut(&mut self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::Color;

    /// 【冻结截图回归】【共享隔离】后台借用必须复用像素，编辑器修改后保持后台原图，独占交付复用已有缓冲，无参数、无返回值
    #[test]
    fn worker_snapshot_survives_editor_mutation_without_an_initial_copy() {
        let mut pixels = Pixmap::new(4, 4).unwrap();
        pixels.fill(Color::from_rgba8(30, 40, 50, 255));
        let original_pointer = pixels.data().as_ptr();
        let mut frozen = FrozenPixmap::from(pixels);
        let worker = frozen.share();
        assert_eq!(worker.data().as_ptr(), original_pointer);
        frozen.fill(Color::from_rgba8(60, 70, 80, 255));
        assert_eq!(&worker.data()[..4], &[30, 40, 50, 255]);
        assert_eq!(&frozen.data()[..4], &[60, 70, 80, 255]);
        let edited_pointer = frozen.data().as_ptr();
        let delivered = frozen.into_pixmap();
        assert_eq!(delivered.data().as_ptr(), edited_pointer);
    }

    /// 【冻结截图回归】【私有目录】无参数，返回项目内磁盘测试目录，不写入用户缓存
    fn directory() -> tempfile::TempDir {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/frozen-source-tests");
        std::fs::create_dir_all(&root).unwrap();
        tempfile::tempdir_in(root).unwrap()
    }

    /// 【冻结截图回归】【缓存共享】无参数，后台共享缓存期间修改必须隔离原图，后台结束后清理唯一原件文件，无返回值
    #[test]
    fn cached_worker_keeps_original_pixels_after_editor_changes() {
        let directory = directory();
        let mut pixels = Pixmap::new(1100, 1100).unwrap();
        pixels.fill(Color::from_rgba8(29, 73, 131, 137));
        let original = pixels.clone();
        let mut frozen = FrozenPixmap(Arc::new(ImageSource::store_in(pixels, directory.path())));
        assert!(frozen.cached_bytes() > 4 * 1024 * 1024);
        let worker = frozen.share();
        assert_eq!(worker.data().as_ptr(), frozen.data().as_ptr());
        let mut patch = Pixmap::new(12, 15).unwrap();
        patch.fill(Color::from_rgba8(90, 41, 73, 87));
        frozen.replace_pixels(&patch, (107, 231));
        assert_eq!(worker.data(), original.data());
        let mut expected = original;
        crate::renderer::paste(&mut expected, &patch, 107, 231);
        assert_eq!(frozen.data(), expected.data());
        let edited_pointer = frozen.data().as_ptr();
        let delivered = frozen.into_pixmap();
        assert_eq!(delivered.data().as_ptr(), edited_pointer);
        drop(worker);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    /// 【冻结截图回归】【独占版本】无参数，独占冻结缓存的区域写回应保持缓存，修改只影响指定像素，无返回值
    #[test]
    fn unique_cached_patch_stays_in_a_private_file() {
        let directory = directory();
        let mut pixels = Pixmap::new(1100, 1100).unwrap();
        pixels.fill(Color::from_rgba8(29, 73, 131, 137));
        let mut expected = pixels.clone();
        let mut frozen = FrozenPixmap(Arc::new(ImageSource::store_in(pixels, directory.path())));
        let mut patch = Pixmap::new(7, 5).unwrap();
        patch.fill(Color::from_rgba8(90, 41, 73, 87));
        frozen.replace_pixels(&patch, (1093, 1095));
        crate::renderer::paste(&mut expected, &patch, 1093, 1095);
        assert_eq!(frozen.data(), expected.data());
        assert!(frozen.cached_bytes() > 4 * 1024 * 1024);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
        drop(frozen);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}
