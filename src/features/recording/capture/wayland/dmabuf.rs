//! 【显存采集】【协议缓冲】按合成器声明的设备、格式和修饰符分配缓冲，失败交由共享内存兼容路径

use super::State;
use shot_media::{Media, Surface};
use std::{os::unix::fs::MetadataExt, path::PathBuf};
use wayland_client::{Connection, Dispatch, QueueHandle, protocol::wl_buffer};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_buffer_params_v1 as params, zwp_linux_dmabuf_feedback_v1 as feedback,
    zwp_linux_dmabuf_v1 as dmabuf,
};

pub(super) struct Buffer {
    pub proxy: wl_buffer::WlBuffer,
    pub surface: Surface,
}
impl Drop for Buffer {
    /// 【显存采集】【缓冲释放】无参数，先销毁协议对象，再回收当前进程持有的 GPU 缓冲
    fn drop(&mut self) {
        self.proxy.destroy();
    }
}

/// 【显存采集】【渲染设备】按原生 dev_t 字节查找同一 DRM 设备的 render 节点，不猜测主显卡
pub(super) fn device(bytes: &[u8]) -> Option<PathBuf> {
    let dev = u64::from_ne_bytes(bytes.try_into().ok()?);
    let directory = std::fs::read_dir("/dev/dri").ok()?;
    for entry in directory.flatten() {
        if entry.metadata().ok()?.rdev() != dev {
            continue;
        }
        if entry.file_name().to_string_lossy().starts_with("renderD") {
            return Some(entry.path());
        }
        let sys = PathBuf::from("/sys/class/drm")
            .join(entry.file_name())
            .join("device/drm");
        for node in std::fs::read_dir(sys).ok()?.flatten() {
            if node.file_name().to_string_lossy().starts_with("renderD") {
                return Some(PathBuf::from("/dev/dri").join(node.file_name()));
            }
        }
    }
    None
}

/// 【显存采集】【缓冲提交】按 manager、state、media 和 qh 创建一个可采集的缓冲，返回带生命周期的协议与显存资源
pub(super) fn allocate(
    manager: &dmabuf::ZwpLinuxDmabufV1,
    state: &State,
    media: &Media,
    qh: &QueueHandle<State>,
) -> Result<(params::ZwpLinuxBufferParamsV1, Surface), String> {
    let device = state
        .device
        .as_deref()
        .ok_or("Compositor did not identify the DMA-BUF device")?;
    let size = state.size.ok_or("Missing DMA-BUF dimensions")?;
    for (format, modifiers) in &state.dma_formats {
        let Ok(surface) = Surface::new(media, device, size, *format, modifiers) else {
            continue;
        };
        let parameters = manager.create_params(qh, ());
        for (index, (fd, offset, stride)) in surface.planes().enumerate() {
            parameters.add(
                fd,
                index as u32,
                offset,
                stride,
                (surface.modifier() >> 32) as u32,
                surface.modifier() as u32,
            );
        }
        parameters.create(
            size.0 as i32,
            size.1 as i32,
            surface.format(),
            params::Flags::empty(),
        );
        return Ok((parameters, surface));
    }
    Err("No compatible DMA-BUF allocation".into())
}

impl Dispatch<feedback::ZwpLinuxDmabufFeedbackV1, ()> for State {
    /// 【显存采集】【设备反馈】按 event 保存默认渲染设备，格式仍使用捕获协议的具体约束
    fn event(
        state: &mut Self,
        _: &feedback::ZwpLinuxDmabufFeedbackV1,
        event: feedback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let feedback::Event::MainDevice { device: bytes } = event {
            state.device = device(&bytes);
        }
    }
}
wayland_client::delegate_noop!(State: ignore dmabuf::ZwpLinuxDmabufV1);
impl Dispatch<params::ZwpLinuxBufferParamsV1, ()> for State {
    /// 【显存采集】【导入结果】按 event 保存缓冲或失败，使用可失败请求避免不兼容驱动终止整个 Wayland 连接
    fn event(
        state: &mut Self,
        _: &params::ZwpLinuxBufferParamsV1,
        event: params::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            params::Event::Created { buffer } => state.dma_created = Some(Some(buffer)),
            params::Event::Failed => state.dma_created = Some(None),
            _ => {}
        }
    }
    wayland_client::event_created_child!(State, params::ZwpLinuxBufferParamsV1, [0 => (wl_buffer::WlBuffer, ())]);
}
