//! 【共享内存池】【槽位分配】基于无密封 RawPool 管理槽位复用、相邻空闲区间合并与服务端释放生命周期

pub mod raw;

#[cfg(test)]
mod tests;

use raw::RawPool;
use smithay_client_toolkit::{globals::ProvidesBoundGlobal, shm::CreatePoolError};
use std::{
    fmt::{self, Display},
    io,
    os::unix::io::{AsRawFd, OwnedFd},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicU8, AtomicUsize, Ordering},
    },
};
use wayland_client::{
    Proxy,
    protocol::{wl_buffer, wl_shm, wl_surface},
};

#[derive(Debug)]
pub enum CreateBufferError {
    Io(io::Error),
    PoolMismatch,
    SlotTooSmall,
}

impl fmt::Display for CreateBufferError {
    /// 【共享内存池】【错误格式化】按缓冲创建失败原因写入描述文本，返回格式化结果
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => Display::fmt(error, f),
            Self::PoolMismatch => f.write_str("incorrect pool for slot"),
            Self::SlotTooSmall => f.write_str("requested buffer size is too large for slot"),
        }
    }
}

impl std::error::Error for CreateBufferError {
    /// 【共享内存池】【底层错误源】无参数，返回系统 I/O 错误源或空值
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::PoolMismatch | Self::SlotTooSmall => None,
        }
    }
}

impl From<io::Error> for CreateBufferError {
    /// 【共享内存池】【错误转换】将系统 I/O 错误包装为 CreateBufferError，返回目标枚举
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug)]
pub enum ActivateSlotError {
    AlreadyActive,
}

impl fmt::Display for ActivateSlotError {
    /// 【共享内存池】【激活错误描述】格式化槽位重复激活错误，返回格式化结果
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyActive => f.write_str("buffer was already active"),
        }
    }
}

impl std::error::Error for ActivateSlotError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FreelistEntry {
    pub(super) offset: usize,
    pub(super) len: usize,
}

#[derive(Debug)]
pub struct SlotPool {
    inner: RawPool,
    free_list: Arc<Mutex<Vec<FreelistEntry>>>,
}

#[derive(Debug)]
pub struct Slot {
    inner: Arc<SlotInner>,
}

#[derive(Debug)]
struct SlotInner {
    free_list: Weak<Mutex<Vec<FreelistEntry>>>,
    offset: usize,
    len: usize,
    active_buffers: AtomicUsize,
    all_refs: AtomicUsize,
}

#[derive(Debug)]
pub struct Buffer {
    buffer: wl_buffer::WlBuffer,
    height: i32,
    stride: i32,
    slot: Slot,
}

#[derive(Debug)]
struct BufferData {
    inner: Arc<SlotInner>,
    state: AtomicU8,
}

impl BufferData {
    const ACTIVE: u8 = 0;
    const INACTIVE: u8 = 1;
    const DESTROY_ON_RELEASE: u8 = 2;
    const DEAD: u8 = 3;
    const RELEASE_SET: u8 = 1;
    const DESTROY_SET: u8 = 2;

    /// 【共享内存池】【引用归还】缓冲转入 DEAD 状态后释放其持有的槽位强引用，无参数和返回值
    fn record_death(&self) {
        drop(Slot {
            inner: self.inner.clone(),
        });
    }
}

impl SlotPool {
    /// 【共享内存池】【实例初始化】按初始字节数 len 与 shm 全局对象建立无密封槽位池，返回槽位池或错误
    pub fn new(
        len: usize,
        shm: &impl ProvidesBoundGlobal<wl_shm::WlShm, 1>,
    ) -> Result<Self, CreatePoolError> {
        let inner = RawPool::new(len.max(64), shm)?;
        let free_list = Arc::new(Mutex::new(vec![FreelistEntry {
            offset: 0,
            len: inner.len(),
        }]));
        Ok(Self { inner, free_list })
    }

    /// 【共享内存池】【新建缓冲】按 width、height、stride 与 format 分配独立槽位并创建 wl_buffer，返回缓冲与可写切片
    pub fn create_buffer(
        &mut self,
        width: i32,
        height: i32,
        stride: i32,
        format: wl_shm::Format,
    ) -> Result<(Buffer, &mut [u8]), CreateBufferError> {
        // 1. 校验几何尺寸并计算所需字节长度
        let len = buffer_byte_len(width, height, stride)?;
        // 2. 分配槽位、构造协议缓冲并返回槽位首段可写切片
        let slot = self.new_slot(len)?;
        let buffer = self.create_buffer_in(&slot, width, height, stride, format)?;
        let canvas = &mut self.raw_data_mut(&slot)[..len];
        Ok((buffer, canvas))
    }

    /// 【共享内存池】【安全画布】按 key 返回空闲槽位或缓冲对应的可写字节切片；服务端占用或跨池时返回空值
    pub fn canvas<'pool>(&'pool mut self, key: &impl CanvasKey) -> Option<&'pool mut [u8]> {
        key.canvas(self)
    }

    /// 【共享内存池】【槽位分配】按请求字节数 len 向上对齐 64 字节并分配槽位，不足时自动扩容，返回新槽位
    pub fn new_slot(&mut self, len: usize) -> io::Result<Slot> {
        let aligned = len
            .max(1)
            .checked_add(63)
            .map(|value| value & !63)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "slot size overflow"))?;
        let offset = self.alloc(aligned)?;
        Ok(Slot {
            inner: Arc::new(SlotInner {
                free_list: Arc::downgrade(&self.free_list),
                offset,
                len: aligned,
                active_buffers: AtomicUsize::new(0),
                all_refs: AtomicUsize::new(1),
            }),
        })
    }

    /// 【共享内存池】【原始切片】按 slot 返回底层映射切片；不属于当前池时返回空切片
    pub fn raw_data_mut(&mut self, slot: &Slot) -> &mut [u8] {
        if slot.inner.free_list.as_ptr() == Arc::as_ptr(&self.free_list) {
            &mut self.inner.mmap()[slot.inner.offset..][..slot.inner.len]
        } else {
            &mut []
        }
    }

    /// 【共享内存池】【槽内建缓冲】在既有 slot 上按 width、height、stride 与 format 创建 wl_buffer，返回缓冲或校验错误
    pub fn create_buffer_in(
        &mut self,
        slot: &Slot,
        width: i32,
        height: i32,
        stride: i32,
        format: wl_shm::Format,
    ) -> Result<Buffer, CreateBufferError> {
        let len = buffer_byte_len(width, height, stride)?;
        if len > slot.inner.len {
            return Err(CreateBufferError::SlotTooSmall);
        }
        if slot.inner.free_list.as_ptr() != Arc::as_ptr(&self.free_list) {
            return Err(CreateBufferError::PoolMismatch);
        }
        let offset = i32::try_from(slot.inner.offset)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "slot offset exceeds i32"))?;
        // 1. 为 BufferData 增加一个槽位引用，在缓冲彻底销毁时通过 record_death 归还
        let slot = slot.clone();
        slot.inner.all_refs.fetch_add(1, Ordering::Relaxed);
        let data = Arc::new(BufferData {
            inner: slot.inner.clone(),
            state: AtomicU8::new(BufferData::INACTIVE),
        });
        let buffer = self
            .inner
            .create_buffer_raw(offset, width, height, stride, format, data);
        Ok(Buffer {
            buffer,
            height,
            stride,
            slot,
        })
    }

    /// 【共享内存池】【偏移分配】从空闲表切分 size 字节，空间不足时倍增底层共享池容量并返回起始偏移
    fn alloc(&mut self, size: usize) -> io::Result<usize> {
        let mut free = self.free_list.lock().unwrap();
        let current_len = self.inner.len();
        if let Some(offset) = take_from_freelist(&mut free, size) {
            return Ok(offset);
        }
        // 1. 计算尾部可复用空闲长度并按至少两倍策略扩容底层共享内存
        let (rv, pop_tail) = match free.last() {
            Some(&FreelistEntry { offset, len }) if offset + len == current_len => {
                (current_len - len, true)
            }
            _ => (current_len, false),
        };
        let needed = rv
            .checked_add(size)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "pool offset overflow"))?;
        let target = needed.max(current_len.saturating_mul(2).min(i32::MAX as usize));
        self.inner.resize(target)?;
        // 2. 更新扩容后的尾部空闲区间
        if pop_tail {
            free.pop();
        }
        if target > needed {
            free.push(FreelistEntry {
                offset: needed,
                len: target - needed,
            });
        }
        Ok(rv)
    }

    /// 【共享内存池】【区间归还】将释放的 offset..offset+len 归还至有序空闲表并合并相邻块，无返回值
    pub(super) fn free(free_list: &Mutex<Vec<FreelistEntry>>, offset: usize, len: usize) {
        let mut free = free_list.lock().unwrap();
        coalesce_into_freelist(&mut free, offset, len);
    }
}

/// 【共享内存池】【空闲切分】从 free 中切出首个不少于 size 的区间，成功时返回起始偏移
pub(super) fn take_from_freelist(free: &mut Vec<FreelistEntry>, size: usize) -> Option<usize> {
    let index = free.iter().position(|entry| entry.len >= size)?;
    let rv = free[index].offset;
    free[index].offset += size;
    free[index].len -= size;
    if free[index].len == 0 {
        free.remove(index);
    }
    Some(rv)
}

/// 【共享内存池】【相邻合并】将 offset 与 len 按升序插入 free 并与前后紧邻空闲块合并，无返回值
pub(super) fn coalesce_into_freelist(
    free: &mut Vec<FreelistEntry>,
    mut offset: usize,
    mut len: usize,
) {
    if len == 0 {
        return;
    }
    let mut merged = Vec::with_capacity(free.len() + 1);
    for &FreelistEntry {
        offset: item_offset,
        len: item_len,
    } in free.iter()
    {
        if item_offset + item_len == offset {
            offset = item_offset;
            len += item_len;
            continue;
        }
        if item_offset == offset + len {
            len += item_len;
            continue;
        }
        if item_offset > offset + len && len != 0 {
            merged.push(FreelistEntry { offset, len });
            len = 0;
        }
        if item_len != 0 {
            merged.push(FreelistEntry {
                offset: item_offset,
                len: item_len,
            });
        }
    }
    if len != 0 {
        merged.push(FreelistEntry { offset, len });
    }
    *free = merged;
}

/// 【共享内存池】【字节核算】校验 width、height 与 stride 合法并返回像素区总字节数
fn buffer_byte_len(width: i32, height: i32, stride: i32) -> io::Result<usize> {
    if width <= 0 || height <= 0 || stride < width.saturating_mul(4) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid Wayland shm buffer dimensions",
        ));
    }
    (height as usize)
        .checked_mul(stride as usize)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Wayland shm buffer overflow"))
}

impl Clone for Slot {
    /// 【共享内存池】【槽位克隆】增加槽位引用计数并返回共享句柄
    fn clone(&self) -> Self {
        let inner = self.inner.clone();
        inner.all_refs.fetch_add(1, Ordering::Relaxed);
        Self { inner }
    }
}

impl Drop for Slot {
    /// 【共享内存池】【槽位释放】最后一个逻辑引用退出时将内存区间归还空闲链表，无返回值
    fn drop(&mut self) {
        if self.inner.all_refs.fetch_sub(1, Ordering::Relaxed) == 1
            && let Some(free_list) = self.inner.free_list.upgrade()
        {
            SlotPool::free(&free_list, self.inner.offset, self.inner.len);
        }
    }
}

impl Drop for SlotInner {
    /// 【共享内存池】【计数断言】底层槽位析构时确认逻辑引用计数已归零，无返回值
    fn drop(&mut self) {
        debug_assert_eq!(*self.all_refs.get_mut(), 0);
    }
}

pub trait CanvasKey {
    /// 【共享内存池】【画布提取】从 pool 提取当前键值对应的空闲可写切片，忙碌或不属于该池时返回空值
    fn canvas<'pool>(&self, pool: &'pool mut SlotPool) -> Option<&'pool mut [u8]>;
}

impl Slot {
    /// 【共享内存池】【活动判定】无参数，返回当前槽位是否存在尚未收到服务端 release 的缓冲
    pub fn has_active_buffers(&self) -> bool {
        self.inner.active_buffers.load(Ordering::Relaxed) != 0
    }

    /// 【共享内存池】【槽位画布】从 pool 提取空闲槽位的可写切片，活动或跨池时返回空值
    pub fn canvas<'pool>(&self, pool: &'pool mut SlotPool) -> Option<&'pool mut [u8]> {
        if self.has_active_buffers() {
            return None;
        }
        if self.inner.free_list.as_ptr() == Arc::as_ptr(&pool.free_list) {
            Some(&mut pool.inner.mmap()[self.inner.offset..][..self.inner.len])
        } else {
            None
        }
    }
}

impl CanvasKey for Slot {
    /// 【共享内存池】【槽位键实现】委托 Slot::canvas 返回空闲槽位切片
    fn canvas<'pool>(&self, pool: &'pool mut SlotPool) -> Option<&'pool mut [u8]> {
        self.canvas(pool)
    }
}

impl Buffer {
    /// 【共享内存池】【表面附着】标记当前缓冲为服务端占用并附着到 surface，重复附着活动缓冲时返回错误
    pub fn attach_to(&self, surface: &wl_surface::WlSurface) -> Result<(), ActivateSlotError> {
        self.activate()?;
        surface.attach(Some(&self.buffer), 0, 0);
        Ok(())
    }

    /// 【共享内存池】【协议句柄】无参数，返回内部 wl_buffer 引用
    pub fn wl_buffer(&self) -> &wl_buffer::WlBuffer {
        &self.buffer
    }

    /// 【共享内存池】【缓冲高度】无参数，返回缓冲像素高度
    pub fn height(&self) -> i32 {
        self.height
    }

    /// 【共享内存池】【缓冲步幅】无参数，返回每行字节步幅
    pub fn stride(&self) -> i32 {
        self.stride
    }

    /// 【共享内存池】【缓冲画布】从 pool 返回与当前缓冲尺寸一致的可写像素切片，占用或跨池时返回空值
    pub fn canvas<'pool>(&self, pool: &'pool mut SlotPool) -> Option<&'pool mut [u8]> {
        let len = (self.height as usize) * (self.stride as usize);
        if self.slot.has_active_buffers() {
            return None;
        }
        if self.slot.inner.free_list.as_ptr() == Arc::as_ptr(&pool.free_list) {
            Some(&mut pool.inner.mmap()[self.slot.inner.offset..][..len])
        } else {
            None
        }
    }

    /// 【共享内存池】【所属槽位】无参数，克隆并返回承载当前缓冲的槽位句柄
    pub fn slot(&self) -> Slot {
        self.slot.clone()
    }

    /// 【共享内存池】【占用登记】将缓冲状态由 INACTIVE 原子切换为 ACTIVE，已活动时返回错误
    pub fn activate(&self) -> Result<(), ActivateSlotError> {
        let data = self.data().expect("WlBuffer ObjectData mismatch");
        match data
            .state
            .fetch_and(!BufferData::RELEASE_SET, Ordering::Relaxed)
        {
            BufferData::INACTIVE => {
                data.inner.active_buffers.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            BufferData::ACTIVE => Err(ActivateSlotError::AlreadyActive),
            _ => unreachable!("invalid BufferData state"),
        }
    }

    /// 【共享内存池】【内部数据】无参数，读取协议对象挂载的 BufferData
    fn data(&self) -> Option<&BufferData> {
        self.buffer.object_data()?.downcast_ref()
    }
}

impl CanvasKey for Buffer {
    /// 【共享内存池】【缓冲键实现】委托 Buffer::canvas 返回缓冲像素切片
    fn canvas<'pool>(&self, pool: &'pool mut SlotPool) -> Option<&'pool mut [u8]> {
        self.canvas(pool)
    }
}

impl Drop for Buffer {
    /// 【共享内存池】【缓冲释放】空闲缓冲立即销毁并归还槽位，在途缓冲延迟到服务端 release 时自动销毁
    fn drop(&mut self) {
        if let Some(data) = self.data() {
            match data
                .state
                .fetch_or(BufferData::DESTROY_SET, Ordering::Relaxed)
            {
                BufferData::ACTIVE => {}
                BufferData::INACTIVE => {
                    data.record_death();
                    self.buffer.destroy();
                }
                _ => unreachable!("invalid BufferData state"),
            }
        }
    }
}

impl wayland_client::backend::ObjectData for BufferData {
    /// 【共享内存池】【释放事件】处理服务端 wl_buffer.release 事件，解除活动占用并在延迟销毁状态下发送 destroy 请求
    fn event(
        self: Arc<Self>,
        handle: &wayland_client::backend::Backend,
        msg: wayland_client::backend::protocol::Message<wayland_client::backend::ObjectId, OwnedFd>,
    ) -> Option<Arc<dyn wayland_client::backend::ObjectData>> {
        debug_assert!(wayland_client::backend::protocol::same_interface(
            msg.sender_id.interface(),
            wl_buffer::WlBuffer::interface()
        ));
        debug_assert!(msg.opcode == 0);

        match self
            .state
            .fetch_or(BufferData::RELEASE_SET, Ordering::Relaxed)
        {
            BufferData::ACTIVE => {
                self.inner.active_buffers.fetch_sub(1, Ordering::Relaxed);
            }
            BufferData::INACTIVE | BufferData::DEAD => {}
            BufferData::DESTROY_ON_RELEASE => {
                // 1. 归还槽位引用并扣减活动计数，随后复用相同对象编号发送无参 destroy 消息
                self.record_death();
                self.inner.active_buffers.fetch_sub(1, Ordering::Relaxed);
                let _ = handle.send_request(msg.map_fd(|fd| fd.as_raw_fd()), None, None);
            }
            _ => unreachable!("invalid BufferData state"),
        }
        None
    }

    /// 【共享内存池】【对象销毁通知】协议层完成回收时无额外操作，无返回值
    fn destroyed(&self, _: wayland_client::backend::ObjectId) {}
}

impl Drop for BufferData {
    /// 【共享内存池】【连接断开兜底】连接关闭导致事件未到达时兜底归还活动计数与槽位引用，无返回值
    fn drop(&mut self) {
        let state = *self.state.get_mut();
        if state == BufferData::ACTIVE || state == BufferData::DESTROY_ON_RELEASE {
            self.inner.active_buffers.fetch_sub(1, Ordering::Relaxed);
        }
        if state != BufferData::DEAD {
            self.record_death();
        }
    }
}
