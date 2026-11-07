//! 【图像原件】【私有文件】唯一持有只读缓存映射及文件，释放读取页不删除仍在使用的原件

use memmap2::{Mmap, MmapOptions};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
use tiny_skia::{IntSize, PixmapRef};

// 【图像原件】【内存介质】编号对应Linux的linux/magic.h，包含tmpfs、ramfs和hugetlbfs
#[cfg(target_os = "linux")]
pub(super) const MEMORY_FILESYSTEMS: [u32; 3] =
    [nix::libc::TMPFS_MAGIC as u32, 0x858458f6, 0x958458f6];

pub(super) struct CacheFile {
    map: Mmap,
    _file: File,
    _directory: tempfile::TempDir,
    release_error: AtomicBool,
}

impl CacheFile {
    /// 【图像原件】【私有存储】按pixels和root创建匿名原件文件及只读映射，返回所有者或文件错误，不复制到另一个完整堆缓冲
    pub(super) fn new(pixels: PixmapRef<'_>, root: &Path) -> std::io::Result<Self> {
        std::fs::create_dir_all(root)?;
        let directory = tempfile::Builder::new()
            .prefix("source-")
            .tempdir_in(root)?;
        let mut file = tempfile::tempfile_in(directory.path())?;
        Self::check_backing(&file)?;
        // 1. 【图像原件】【完整写入】直接分批写入原始预乘字节，同步并回收已完成批次，完成持久化后才建立只读视图
        super::cache_write::write(&mut file, pixels.data())?;
        Self::finish(file, directory, pixels.data().len())
    }

    /// 【图像原件】【磁盘校验】按file检查实际文件系统，返回结果，拒绝仍把完整原件保留在内存的文件系统
    fn check_backing(file: &File) -> std::io::Result<()> {
        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsRawFd;
            let mut status = std::mem::MaybeUninit::<nix::libc::statfs>::uninit();
            // 1. 【图像原件】【实际介质】只读取当前文件所属介质，不根据目录名称猜测是否有磁盘后端
            let result = unsafe { nix::libc::fstatfs(file.as_raw_fd(), status.as_mut_ptr()) };
            if result != 0 {
                return Err(std::io::Error::last_os_error());
            }
            let status = unsafe { status.assume_init() };
            if MEMORY_FILESYSTEMS.contains(&(status.f_type as u32)) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "original cache requires a disk-backed filesystem",
                ));
            }
        }
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::io::AsRawHandle;
            if unsafe { windows_sys::Win32::Storage::FileSystem::GetFileType(file.as_raw_handle()) }
                != windows_sys::Win32::Storage::FileSystem::FILE_TYPE_DISK
            {
                return Err(std::io::Error::other("original cache requires a disk file"));
            }
        }
        Ok(())
    }

    /// 【图像原件】【映射完成】将已经写好的file与directory转为只读所有者，bytes指定完整长度，返回映射或文件错误
    fn finish(file: File, directory: tempfile::TempDir, bytes: usize) -> std::io::Result<Self> {
        file.sync_data()?;
        // 2. 【图像原件】【映射不变性】文件匿名且仅本对象持有，映射建立后不再写入，也不暴露可写文件别名
        let map = unsafe { MmapOptions::new().len(bytes).map(&file)? };
        // 3. 【图像原件】【按需读取】预览和局部绘制按像素窗口访问，禁止自动预读不参与本批绘制的整段原件
        #[cfg(unix)]
        map.advise(memmap2::Advice::Random)?;
        let source = Self {
            map,
            _file: file,
            _directory: directory,
            release_error: AtomicBool::new(false),
        };
        source.release_pages();
        Ok(source)
    }

    /// 【图像原件】【片段替换】按size、patch和position生成新的只读原件，返回新所有者；旧文件与所有既有视图保持不变
    pub(super) fn replaced(
        &self,
        size: IntSize,
        patch: PixmapRef<'_>,
        position: (i32, i32),
    ) -> std::io::Result<Self> {
        let root = self._directory.path().parent().unwrap();
        let directory = tempfile::Builder::new()
            .prefix("source-")
            .tempdir_in(root)?;
        let mut file = tempfile::tempfile_in(directory.path())?;
        // 1. 【图像原件】【独立版本】优先使用文件范围复制，禁止修改既有只读映射的文件，也不创建完整RGBA堆副本
        Self::check_backing(&file)?;
        self.copy_file(&mut file)?;
        super::region_copy::patch_rows(size, patch, position, |offset, bytes| {
            file.seek(SeekFrom::Start(offset as u64))?;
            file.write_all(bytes)
        })?;
        self.release_pages();
        Self::finish(file, directory, self.map.len())
    }

    /// 【图像原件】【文件复制】复制当前原件到target，返回文件结果，平台不支持范围复制时仅使用4MiB工作缓冲
    fn copy_file(&self, target: &mut File) -> std::io::Result<()> {
        let mut offset = 0_usize;
        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsRawFd;
            let mut input: nix::libc::off64_t = 0;
            let mut output: nix::libc::off64_t = 0;
            // 1. 【图像原件】【原生复制】偏移量只属于本次操作，复制不会修改原文件，也不借用映射中的大像素片段
            while offset < self.map.len() {
                let count = (4 * 1024 * 1024).min(self.map.len() - offset);
                let copied = unsafe {
                    nix::libc::copy_file_range(
                        self._file.as_raw_fd(),
                        &mut input,
                        target.as_raw_fd(),
                        &mut output,
                        count,
                        0,
                    )
                };
                if copied <= 0 {
                    break;
                }
                offset += copied as usize;
            }
        }
        if offset == self.map.len() {
            return Ok(());
        }
        // 2. 【图像原件】【有界兼容】不支持原生复制的剩余部分使用固定工作缓冲，不创建完整原件副本
        let mut source = self._file.try_clone()?;
        source.seek(SeekFrom::Start(offset as u64))?;
        target.seek(SeekFrom::Start(offset as u64))?;
        let mut buffer = vec![0; 4 * 1024 * 1024];
        while offset < self.map.len() {
            let limit = buffer.len().min(self.map.len() - offset);
            source.read_exact(&mut buffer[..limit])?;
            target.write_all(&buffer[..limit])?;
            offset += limit;
        }
        Ok(())
    }

    /// 【图像原件】【映射读取】无参数，返回完整只读预乘字节；所有者保留期间地址与文件内容不变
    pub(super) fn data(&self) -> &[u8] {
        &self.map
    }

    /// 【图像原件】【页回收】无参数，归还可从私有文件重新读取的页，错误仅记录一次，不改变像素或解除地址映射
    pub(super) fn release_pages(&self) {
        if let Err(error) = self.reclaim()
            && !self.release_error.swap(true, Ordering::Relaxed)
        {
            log::error!("【图像原件】【页回收】{error}");
        }
    }

    /// 【图像原件】【平台回收】无参数，Unix释放只读映射页，Linux同时释放干净文件缓存，返回原生结果
    fn reclaim(&self) -> std::io::Result<()> {
        #[cfg(unix)]
        // 1. 【图像原件】【只读页】没有私有修改页，重新读取始终来自已经同步的唯一原件文件，地址和字节契约保持不变
        unsafe {
            self.map
                .unchecked_advise(memmap2::UncheckedAdvice::DontNeed)?;
        }
        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsRawFd;
            // 2. 【图像原件】【文件缓存】写入已同步，映射页已归还，避免把原件内存转移为仍常驻的文件缓存
            let result = unsafe {
                nix::libc::posix_fadvise(
                    self._file.as_raw_fd(),
                    0,
                    0,
                    nix::libc::POSIX_FADV_DONTNEED,
                )
            };
            if result != 0 {
                return Err(std::io::Error::from_raw_os_error(result));
            }
        }
        Ok(())
    }
}
