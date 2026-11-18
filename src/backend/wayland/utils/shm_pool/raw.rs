//! 【共享内存池】【无密封文件】使用不启用密封标志的匿名文件承载 wl_shm_pool，阻断合成器触发 UDMABUF 与 FOLL_PIN

use memmap2::MmapMut;
use nix::{
    errno::Errno,
    sys::memfd::{MFdFlags, memfd_create},
};
use smithay_client_toolkit::{globals::ProvidesBoundGlobal, shm::CreatePoolError};
use std::{
    fs::File,
    io,
    ops::Deref,
    os::unix::io::{AsFd, BorrowedFd, OwnedFd},
    sync::Arc,
};
use wayland_client::{
    Dispatch, Proxy, QueueHandle, WEnum,
    backend::ObjectData,
    protocol::{wl_buffer, wl_shm, wl_shm_pool},
};

#[derive(Debug)]
pub struct RawPool {
    pool: DestroyOnDropPool,
    len: usize,
    mem_file: File,
    mmap: MmapMut,
}

impl RawPool {
    /// 【共享内存池】【底层创建】按初始字节数 len 与已绑定 shm 创建无密封共享内存池，返回实例或创建错误
    pub fn new(
        len: usize,
        shm: &impl ProvidesBoundGlobal<wl_shm::WlShm, 1>,
    ) -> Result<Self, CreatePoolError> {
        let shm = shm.bound_global()?;
        // 1. 校验协议尺寸上限，避免向服务端发送溢出后的负字节数
        let protocol_len = protocol_pool_size(len)?;
        // 2. 创建不含文件密封能力的匿名内存描述符并映射可写区域
        let shm_fd = create_unsealed_shm_fd()?;
        let mem_file = File::from(shm_fd);
        mem_file.set_len(len as u64)?;
        let pool = shm
            .send_constructor(
                wl_shm::Request::CreatePool {
                    fd: mem_file.as_fd(),
                    size: protocol_len,
                },
                Arc::new(ShmPoolData),
            )
            .unwrap_or_else(|_| Proxy::inert(shm.backend().clone()));
        let mmap = unsafe { MmapMut::map_mut(&mem_file)? };
        Ok(Self {
            pool: DestroyOnDropPool(pool),
            len,
            mem_file,
            mmap,
        })
    }

    /// 【共享内存池】【底层扩容】按目标字节数 size 单向扩展底层文件、协议池与内存映射，返回系统错误
    pub fn resize(&mut self, size: usize) -> io::Result<()> {
        if size > self.len {
            // 1. 先扩展底层文件与协议池，再刷新客户端可写映射
            let protocol_size = protocol_pool_size(size)?;
            self.mem_file.set_len(size as u64)?;
            self.pool.resize(protocol_size);
            self.mmap = unsafe { MmapMut::map_mut(&self.mem_file)? };
            self.len = size;
        }
        Ok(())
    }

    /// 【共享内存池】【映射借用】无参数，返回底层共享内存的可变字节映射
    pub fn mmap(&mut self) -> &mut MmapMut {
        &mut self.mmap
    }

    /// 【共享内存池】【当前容量】无参数，返回当前共享内存池的总字节长度
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// 【共享内存池】【类型化建缓冲】按偏移、宽高、步幅、格式与队列句柄创建 wl_buffer，返回协议缓冲对象
    pub fn create_buffer<D, U>(
        &mut self,
        offset: i32,
        width: i32,
        height: i32,
        stride: i32,
        format: wl_shm::Format,
        udata: U,
        qh: &QueueHandle<D>,
    ) -> wl_buffer::WlBuffer
    where
        D: Dispatch<wl_buffer::WlBuffer, U> + 'static,
        U: Send + Sync + 'static,
    {
        self.pool
            .create_buffer(offset, width, height, stride, format, qh, udata)
    }

    /// 【共享内存池】【原始对象建缓冲】按偏移、宽高、步幅、格式与自定义 ObjectData 创建 wl_buffer，返回协议缓冲对象
    pub fn create_buffer_raw(
        &mut self,
        offset: i32,
        width: i32,
        height: i32,
        stride: i32,
        format: wl_shm::Format,
        data: Arc<dyn ObjectData + 'static>,
    ) -> wl_buffer::WlBuffer {
        self.pool
            .send_constructor(
                wl_shm_pool::Request::CreateBuffer {
                    offset,
                    width,
                    height,
                    stride,
                    format: WEnum::Value(format),
                },
                data,
            )
            .unwrap_or_else(|_| Proxy::inert(self.pool.backend().clone()))
    }
}

impl AsFd for RawPool {
    /// 【共享内存池】【描述符借用】无参数，返回底层共享文件的借用描述符
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.mem_file.as_fd()
    }
}

/// 【共享内存池】【尺寸转换】校验 size 不超过 wl_shm 协议允许的正 32 位整数上限，返回协议长度或输入错误
pub(super) fn protocol_pool_size(size: usize) -> io::Result<i32> {
    i32::try_from(size)
        .ok()
        .filter(|&value| value > 0)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "shared memory pool size exceeds wl_shm bounds",
            )
        })
}

/// 【共享内存池】【无密封描述符】创建仅启用 MFD_CLOEXEC 的匿名内存描述符，避免 F_SEAL_SHRINK 触发合成器 UDMABUF 导入，返回自有描述符
pub(super) fn create_unsealed_shm_fd() -> io::Result<OwnedFd> {
    loop {
        // 1. 仅传入 MFD_CLOEXEC，显式省略 MFD_ALLOW_SEALING 与 F_SEAL_SHRINK，使合成器使用标准 mmap 共享映射
        match memfd_create(c"mark-shot-shm", MFdFlags::MFD_CLOEXEC) {
            Ok(fd) => return Ok(fd),
            Err(Errno::EINTR) => continue,
            // 2. 极少数不支持 memfd_create 的环境回退到 /dev/shm 未链接匿名临时文件
            Err(Errno::ENOSYS) => {
                let file = tempfile::tempfile_in("/dev/shm")?;
                return Ok(OwnedFd::from(file));
            }
            Err(err) => return Err(io::Error::from(err)),
        }
    }
}

#[derive(Debug)]
struct DestroyOnDropPool(wl_shm_pool::WlShmPool);

impl Deref for DestroyOnDropPool {
    type Target = wl_shm_pool::WlShmPool;

    /// 【共享内存池】【协议对象解引用】无参数，返回内部 wl_shm_pool 引用
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for DestroyOnDropPool {
    /// 【共享内存池】【协议销毁】共享池释放时通知服务端销毁 wl_shm_pool，无参数和返回值
    fn drop(&mut self) {
        self.0.destroy();
    }
}

#[derive(Debug)]
struct ShmPoolData;

impl ObjectData for ShmPoolData {
    /// 【共享内存池】【事件占位】wl_shm_pool 协议不产生事件，收到异常事件时直接终止断言
    fn event(
        self: Arc<Self>,
        _: &wayland_client::backend::Backend,
        _: wayland_client::backend::protocol::Message<wayland_client::backend::ObjectId, OwnedFd>,
    ) -> Option<Arc<dyn ObjectData + 'static>> {
        unreachable!("wl_shm_pool has no events")
    }

    /// 【共享内存池】【销毁回调】协议对象销毁通知无需额外状态清理，无返回值
    fn destroyed(&self, _: wayland_client::backend::ObjectId) {}
}
