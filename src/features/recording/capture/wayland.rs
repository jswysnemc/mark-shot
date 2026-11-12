//! 【Wayland录制】【持久采集】复用连接、协议会话和共享缓冲，不为每帧重新建立截图后端

mod dispatch;
mod dmabuf;
mod protocol;

use super::{Frame, Source};
use crate::types::Output;
use memmap2::MmapMut;
use std::{
    fs::File,
    time::{Duration, Instant},
};
use wayland_client::{
    Connection, EventQueue,
    globals::registry_queue_init,
    protocol::{wl_buffer, wl_shm},
};

pub(super) struct Buffer {
    pub proxy: wl_buffer::WlBuffer,
    pub mapping: MmapMut,
    pub _file: File,
}

impl Drop for Buffer {
    /// 【Wayland录制】【缓冲释放】无参数，解除服务器对象后释放共享映射
    fn drop(&mut self) {
        self.proxy.destroy();
    }
}

pub(super) struct State {
    pub device: Option<std::path::PathBuf>,
    pub dma_formats: Vec<(u32, Vec<u64>)>,
    pub dma_created: Option<Option<wl_buffer::WlBuffer>>,
    pub size: Option<(u32, u32)>,
    pub stride: u32,
    pub formats: Vec<wl_shm::Format>,
    pub constraints: bool,
    pub ready: bool,
    pub failure: Option<String>,
    pub inverted: bool,
    pub transform: wayland_client::protocol::wl_output::Transform,
}

impl Default for State {
    /// 【Wayland录制】【空状态】无参数，返回尚未收到缓冲约束的正常方向状态
    fn default() -> Self {
        Self {
            device: None,
            dma_formats: Vec::new(),
            dma_created: None,
            size: None,
            stride: 0,
            formats: Vec::new(),
            constraints: false,
            ready: false,
            failure: None,
            inverted: false,
            transform: wayland_client::protocol::wl_output::Transform::Normal,
        }
    }
}

pub(super) struct Capture {
    media: shot_media::Media,
    dma_manager: Option<wayland_protocols::wp::linux_dmabuf::zv1::client::zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1>,
    dma_feedback: Option<wayland_protocols::wp::linux_dmabuf::zv1::client::zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1>,
    dma_buffer: Option<dmabuf::Buffer>,
    dma_current: Option<dmabuf::Buffer>,
    dma_disabled: bool,
    connection: Connection,
    queue: EventQueue<State>,
    state: State,
    protocol: protocol::Protocol,
    shm: wl_shm::WlShm,
    buffer: Option<Buffer>,
    bgr: bool,
    pixels: Vec<u8>,
    output_size: (u32, u32),
    pixel_size: (u32, u32),
    active: bool,
    deadline: Option<Instant>,
}

impl Capture {
    /// 【Wayland录制】【协议探测】按 connection 检查直接捕获协议，返回实际能力，不使用桌面名称
    pub fn supported(connection: &Connection) -> bool {
        crate::backend::wayland::capture::image_copy::supported(connection)
            || crate::backend::wayland::capture::screencopy::supported(connection)
    }

    /// 【Wayland录制】【持久初始化】按 connection、output 与 cursor 创建协议与单个复用缓冲，返回错误
    pub fn new(
        connection: Connection,
        output: Output,
        cursor: bool,
        media: &shot_media::Media,
        hardware: bool,
    ) -> Result<Self, String> {
        let (globals, mut queue) =
            registry_queue_init::<State>(&connection).map_err(|e| e.to_string())?;
        let qh = queue.handle();
        let dma_manager: Option<
            wayland_protocols::wp::linux_dmabuf::zv1::client::zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
        > = globals.bind(&qh, 4..=5, ()).ok();
        let dma_feedback = dma_manager
            .as_ref()
            .map(|manager| manager.get_default_feedback(&qh, ()));
        let shm = globals
            .bind(&qh, 1..=1, ())
            .map_err(|e| format!("wl_shm: {e}"))?;
        let protocol = protocol::Protocol::new(&globals, &qh, &output, cursor)?;
        let mut state = State {
            transform: output.info.transform,
            ..Default::default()
        };
        queue.roundtrip(&mut state).map_err(|e| e.to_string())?;
        Ok(Self {
            media: media.clone(),
            dma_manager,
            dma_feedback,
            dma_buffer: None,
            dma_current: None,
            dma_disabled: !hardware
                || output.info.transform != wayland_client::protocol::wl_output::Transform::Normal,
            connection,
            queue,
            state,
            protocol,
            shm,
            buffer: None,
            bgr: false,
            pixels: Vec::new(),
            output_size: (0, 0),
            pixel_size: (0, 0),
            active: false,
            deadline: None,
        })
    }

    /// 【Wayland录制】【事件等待】按 timeout 分派协议，返回后不超过单次等待预算
    fn dispatch(&mut self, timeout: Duration) -> Result<(), String> {
        use rustix::event::{PollFd, PollFlags, poll};
        self.queue
            .dispatch_pending(&mut self.state)
            .map_err(|e| e.to_string())?;
        self.connection.flush().map_err(|e| e.to_string())?;
        let Some(guard) = self.queue.prepare_read() else {
            return Ok(());
        };
        let fd = guard.connection_fd();
        let mut fds = [PollFd::new(&fd, PollFlags::IN)];
        let timeout = rustix::time::Timespec::try_from(timeout).map_err(|e| e.to_string())?;
        poll(&mut fds, Some(&timeout)).map_err(|e| e.to_string())?;
        if fds[0].revents().contains(PollFlags::IN) {
            guard.read().map_err(|e| e.to_string())?;
        } else {
            drop(guard);
        }
        self.queue
            .dispatch_pending(&mut self.state)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 【Wayland录制】【共享分配】无参数，约束确定后建立或复用共享缓冲，尺寸变化明确拒绝
    fn allocate(&mut self) -> Result<(), String> {
        let size = self
            .state
            .size
            .ok_or("capture did not announce dimensions")?;
        let length = usize::try_from(u64::from(self.state.stride) * u64::from(size.1))
            .map_err(|e| e.to_string())?;
        if size.0 == 0
            || size.1 == 0
            || length > 128 * 1024 * 1024
            || self.state.stride < size.0.saturating_mul(4)
        {
            return Err("capture exceeds recording memory budget".into());
        }
        if !self.dma_disabled {
            if self.dma_buffer.is_some() {
                if self.output_size != size {
                    return Err("DMA-BUF source dimensions changed".into());
                }
                return Ok(());
            }
            match self.allocate_gpu() {
                Ok(()) => {
                    self.output_size = size;
                    return Ok(());
                }
                Err(error) => {
                    log::info!("【显存采集】【兼容回退】{error}");
                    self.dma_disabled = true;
                    self.dma_current = None;
                }
            }
        }
        if let Some(buffer) = &self.buffer {
            if buffer.mapping.len() != length || self.output_size != size {
                return Err("录制尺寸已变化 / Recording dimensions changed".into());
            }
            return Ok(());
        }
        let format = [
            wl_shm::Format::Xrgb8888,
            wl_shm::Format::Argb8888,
            wl_shm::Format::Xbgr8888,
            wl_shm::Format::Abgr8888,
        ]
        .into_iter()
        .find(|format| self.state.formats.contains(format))
        .ok_or("no supported capture pixel format")?;
        self.bgr = matches!(format, wl_shm::Format::Xrgb8888 | wl_shm::Format::Argb8888);
        let file = tempfile::tempfile().map_err(|e| e.to_string())?;
        file.set_len(length as u64).map_err(|e| e.to_string())?;
        use std::os::fd::AsFd;
        let mapping = unsafe { MmapMut::map_mut(&file) }.map_err(|e| e.to_string())?;
        let pool = self
            .shm
            .create_pool(file.as_fd(), length as i32, &self.queue.handle(), ());
        let proxy = pool.create_buffer(
            0,
            size.0 as i32,
            size.1 as i32,
            self.state.stride as i32,
            format,
            &self.queue.handle(),
            (),
        );
        pool.destroy();
        self.buffer = Some(Buffer {
            proxy,
            mapping,
            _file: file,
        });
        self.output_size = size;
        Ok(())
    }

    /// 【显存采集】【有界分配】无参数，等待合成器确认一次 GPU 导入，失败允许回退，不触发协议致命错误
    fn allocate_gpu(&mut self) -> Result<(), String> {
        let manager = self
            .dma_manager
            .as_ref()
            .ok_or("Linux DMA-BUF protocol unavailable")?;
        self.state.dma_created = None;
        let (parameters, surface) =
            dmabuf::allocate(manager, &self.state, &self.media, &self.queue.handle())?;
        let deadline = Instant::now() + Duration::from_millis(500);
        let result = loop {
            if let Some(result) = self.state.dma_created.take() {
                break result.ok_or_else(|| "Compositor rejected DMA-BUF".to_string());
            }
            if Instant::now() > deadline {
                break Err("DMA-BUF creation timed out".into());
            }
            if let Err(error) = self.dispatch(Duration::from_millis(10)) {
                break Err(error);
            }
        };
        parameters.destroy();
        self.dma_buffer = Some(dmabuf::Buffer {
            proxy: result?,
            surface,
        });
        Ok(())
    }

    /// 【Wayland录制】【方向还原】无参数，将带行距及输出变换的共享像素复制到复用 RGBA，返回实际尺寸
    fn pixels(&mut self) -> (u32, u32) {
        use wayland_client::protocol::wl_output::Transform as T;
        let (width, height) = self.output_size;
        let rotated = matches!(
            self.state.transform,
            T::_90 | T::_270 | T::Flipped90 | T::Flipped270
        );
        let (out_width, out_height) = if rotated {
            (height, width)
        } else {
            (width, height)
        };
        self.pixels
            .resize(out_width as usize * out_height as usize * 4, 255);
        let mapping = &self.buffer.as_ref().unwrap().mapping;
        for y in 0..out_height {
            for x in 0..out_width {
                let (sx, mut sy) = match self.state.transform {
                    T::_90 => (width - 1 - y, x),
                    T::_180 => (width - 1 - x, height - 1 - y),
                    T::_270 => (y, height - 1 - x),
                    T::Flipped => (width - 1 - x, y),
                    T::Flipped90 => (y, x),
                    T::Flipped180 => (x, height - 1 - y),
                    T::Flipped270 => (width - 1 - y, height - 1 - x),
                    _ => (x, y),
                };
                if self.state.inverted {
                    sy = height - 1 - sy;
                }
                let source = sy as usize * self.state.stride as usize + sx as usize * 4;
                let target = (y as usize * out_width as usize + x as usize) * 4;
                let channels = if self.bgr { [2, 1, 0] } else { [0, 1, 2] };
                for (channel, from) in channels.into_iter().enumerate() {
                    self.pixels[target + channel] = mapping[source + from];
                }
            }
        }
        (out_width, out_height)
    }
}

impl Source for Capture {
    /// 【Wayland录制】【事件提前推进】无参数，协议请求和完成事件需在目标帧期限前分派
    fn asynchronous(&self) -> bool {
        true
    }
    /// 【Wayland录制】【有界取帧】按 timeout 分派一次采集，只有完成后才交付借用像素
    fn next(&mut self, timeout: Duration) -> Result<bool, String> {
        if !self.active {
            self.state.ready = false;
            self.state.inverted = false;
            self.protocol.request(&mut self.state, &self.queue.handle());
            self.active = true;
            self.deadline = Some(Instant::now() + Duration::from_secs(5));
        }
        self.dispatch(timeout)?;
        if let Some(error) = self.state.failure.take() {
            return Err(error);
        }
        if self.pixels.is_empty()
            && self.dma_current.is_none()
            && self
                .deadline
                .is_some_and(|deadline| Instant::now() > deadline)
        {
            return Err("Wayland recording frame timed out".into());
        }
        if self.state.constraints && !self.protocol.copying() {
            self.allocate()?;
            self.protocol.copy(
                self.dma_buffer
                    .as_ref()
                    .map(|buffer| &buffer.proxy)
                    .unwrap_or_else(|| &self.buffer.as_ref().unwrap().proxy),
                self.output_size,
                &self.queue.handle(),
            );
            self.connection.flush().map_err(|e| e.to_string())?;
        }
        if !self.state.ready {
            return Ok(false);
        }
        if self.dma_buffer.is_some()
            && (self.state.inverted
                || self.state.transform != wayland_client::protocol::wl_output::Transform::Normal)
        {
            self.prefer_cpu();
            return Ok(false);
        }
        self.protocol.complete();
        self.active = false;
        self.deadline = None;
        if self.dma_buffer.is_some() {
            std::mem::swap(&mut self.dma_buffer, &mut self.dma_current);
            self.pixel_size = self.output_size;
        } else {
            self.pixel_size = self.pixels();
        }
        Ok(true)
    }
    /// 【Wayland录制】【静态画面】无参数，返回已完成像素，可在等待 damage 时延续时长
    fn frame(&self) -> Option<Frame<'_>> {
        if let Some(current) = &self.dma_current {
            return Some(Frame {
                pixels: &[],
                gpu: Some(&current.surface),
                size: self.pixel_size,
                stride: 0,
            });
        }
        if self.pixels.is_empty() {
            return None;
        }
        let size = self.pixel_size;
        Some(Frame {
            gpu: None,
            pixels: &self.pixels,
            size,
            stride: size.0 as usize * 4,
        })
    }
    /// 【Wayland录制】【暂停状态】按 paused 取消未完成帧，恢复时重新请求新画面
    fn pause(&mut self, _paused: bool) -> Result<(), String> {
        self.protocol.complete();
        self.active = false;
        self.deadline = None;
        self.connection.flush().map_err(|e| e.to_string())
    }
    /// 【Wayland录制】【能力名称】无参数，返回实际绑定协议及内存类型
    fn name(&self) -> &'static str {
        if self.dma_current.is_some() {
            "Wayland DMA-BUF / VAAPI"
        } else {
            self.protocol.name()
        }
    }
    /// 【显存采集】【软件切换】无参数，取消当前采集并释放 GPU 缓冲，下一次按共享内存重新取帧
    fn prefer_cpu(&mut self) -> bool {
        self.protocol.complete();
        self.active = false;
        self.dma_disabled = true;
        self.dma_buffer = None;
        self.dma_current = None;
        true
    }
}

impl Drop for Capture {
    /// 【显存采集】【反馈释放】无参数，结束短期帧后销毁设备反馈和管理器，字段随后回收显存
    fn drop(&mut self) {
        self.protocol.complete();
        if let Some(feedback) = &self.dma_feedback {
            feedback.destroy();
        }
        if let Some(manager) = &self.dma_manager {
            manager.destroy();
        }
    }
}
