//! 【主选择区】【原生会话】按需绑定截图现有连接，持有来源、座席授权与有界非阻塞传输

mod transfer;

use super::OverlayState;
use crate::types::OverlayEvent;
use smithay_client_toolkit::{
    data_device_manager::WritePipe,
    primary_selection::{
        PrimarySelectionManagerState,
        device::{PrimarySelectionDevice, PrimarySelectionDeviceHandler},
        selection::{PrimarySelectionSource, PrimarySelectionSourceHandler},
    },
};
use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};
use wayland_client::{Connection, QueueHandle, globals::GlobalList, protocol::wl_seat::WlSeat};
use wayland_protocols::wp::primary_selection::zv1::client::{
    zwp_primary_selection_device_v1::ZwpPrimarySelectionDeviceV1,
    zwp_primary_selection_source_v1::ZwpPrimarySelectionSourceV1,
};

const MIMES: [&str; 3] = ["text/plain;charset=utf-8", "text/plain", "UTF8_STRING"];

struct Owned {
    source: PrimarySelectionSource,
    seat: WlSeat,
    text: Arc<[u8]>,
}

#[derive(Default)]
pub(in crate::backend::wayland::overlay) struct PrimarySelection {
    initialized: bool,
    manager: Option<PrimarySelectionManagerState>,
    devices: Vec<PrimarySelectionDevice>,
    owned: Vec<Owned>,
    inputs: VecDeque<(WlSeat, u32)>,
    current: Option<(WlSeat, u32)>,
    incoming: Option<transfer::Incoming>,
    outgoing: Vec<transfer::Outgoing>,
}

impl PrimarySelection {
    /// 【主选择区】【按钮授权】按 seat 和 serial 排队原始按钮授权，无返回值，释放序列与菜单抓取分开保存
    pub fn button(&mut self, seat: WlSeat, serial: u32) {
        self.inputs.push_back((seat, serial));
    }

    /// 【主选择区】【事件授权】弹出当前应用按钮对应的座席和序列，无参数、无返回值，不复用未来排队事件
    pub fn advance(&mut self) {
        self.current = self.inputs.pop_front();
    }

    /// 【主选择区】【座席绑定】按 seat 与 qh 绑定已启用协议的设备，无返回值，重复能力通知不创建重复对象
    pub fn seat(&mut self, seat: &WlSeat, qh: &QueueHandle<OverlayState>) {
        if let Some(manager) = &self.manager
            && !self
                .devices
                .iter()
                .any(|device| device.data().seat() == seat)
        {
            self.devices.push(manager.get_selection_device(qh, seat));
        }
    }

    /// 【主选择区】【座席释放】移除 seat 的来源与设备，无返回值，当前粘贴及授权同时取消
    pub fn remove(&mut self, seat: &WlSeat) {
        self.devices.retain(|device| device.data().seat() != seat);
        self.owned.retain(|owned| &owned.seat != seat);
        self.inputs.retain(|(owner, _)| owner != seat);
        if self
            .current
            .as_ref()
            .is_some_and(|(owner, _)| owner == seat)
        {
            self.current = None;
            self.incoming = None;
        }
    }

    /// 【主选择区】【协议启用】按 globals、seats 与 qh 按需绑定，返回是否支持，普通静止截图不调用此方法
    pub fn enable(
        &mut self,
        globals: &GlobalList,
        seats: impl Iterator<Item = WlSeat>,
        qh: &QueueHandle<OverlayState>,
    ) -> bool {
        if !self.initialized {
            self.initialized = true;
            self.manager = PrimarySelectionManagerState::bind(globals, qh).ok();
        }
        for seat in seats {
            self.seat(&seat, qh);
        }
        self.manager.is_some()
    }

    /// 【主选择区】【来源设置】按 text 和 qh 取当前按钮授权设置来源，返回是否实际设置，空选择不覆盖已有正文
    pub fn set(&mut self, text: &str, qh: &QueueHandle<OverlayState>) -> bool {
        let Some((seat, serial)) = &self.current else {
            return false;
        };
        let Some(device) = self
            .devices
            .iter()
            .find(|device| device.data().seat() == seat)
        else {
            return false;
        };
        let Some(manager) = &self.manager else {
            return false;
        };
        if text.is_empty() {
            return false;
        }
        let source = manager.create_selection_source(qh, MIMES);
        source.set_selection(device, *serial);
        self.owned.retain(|owned| &owned.seat != seat);
        self.owned.push(Owned {
            source,
            seat: seat.clone(),
            text: text.as_bytes().into(),
        });
        true
    }

    /// 【主选择区】【粘贴请求】按 id 与 limit 接收当前授权座席的主选择，返回即时完成或待管道就绪的状态
    pub fn request(&mut self, id: u64, limit: usize) -> Option<Option<String>> {
        self.incoming = None;
        let Some((seat, _)) = self.current.as_ref() else {
            return Some(None);
        };
        if let Some(owned) = self.owned.iter().find(|owned| &owned.seat == seat) {
            return Some(Some(
                String::from_utf8_lossy(&owned.text[..owned.text.len().min(limit)]).into_owned(),
            ));
        }
        let Some(device) = self
            .devices
            .iter()
            .find(|device| device.data().seat() == seat)
        else {
            return Some(None);
        };
        let Some(offer) = device.data().selection_offer() else {
            return Some(None);
        };
        let mime = offer.with_mime_types(|offered| {
            MIMES
                .iter()
                .find(|mime| offered.iter().any(|value| value == **mime))
                .map(|mime| (*mime).to_owned())
        });
        let Some(mime) = mime else { return Some(None) };
        self.incoming = offer
            .receive(mime)
            .ok()
            .and_then(|pipe| transfer::Incoming::new(id, pipe, limit.min(131071)).ok());
        if self.incoming.is_some() {
            None
        } else {
            Some(None)
        }
    }

    /// 【主选择区】【传输轮询】读取或发送已就绪正文，将接收完成写入 events，无返回值，不循环等待管道
    pub fn progress(&mut self, events: &mut VecDeque<OverlayEvent>) {
        if let Some(incoming) = &mut self.incoming
            && let Some(text) = incoming.progress()
        {
            events.push_back(OverlayEvent::PrimaryText {
                id: incoming.id,
                text,
            });
            self.incoming = None;
        }
        self.outgoing.retain_mut(|outgoing| !outgoing.progress());
    }

    /// 【主选择区】【取消接收】取消 id 对应活动管道，无返回值，已经更新的请求保持不变
    pub fn cancel(&mut self, id: u64) {
        if self
            .incoming
            .as_ref()
            .is_some_and(|incoming| incoming.id == id)
        {
            self.incoming = None;
        }
    }

    /// 【主选择区】【最早期限】返回全部活动传输的最早剩余时间，无活动传输时为空，不增加空闲轮询
    pub fn timeout(&self) -> Option<Duration> {
        self.incoming
            .iter()
            .map(|item| item.deadline)
            .chain(self.outgoing.iter().map(|item| item.deadline))
            .min()
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
    }

    /// 【主选择区】【管道监听】为 fds 增加接收和发送方向，不返回值，只监听本会话持有的文件描述符
    pub fn poll_fds<'a>(&'a self, fds: &mut Vec<rustix::event::PollFd<'a>>) {
        use rustix::event::{PollFd, PollFlags};
        if let Some(incoming) = &self.incoming {
            fds.push(PollFd::new(&incoming.pipe, PollFlags::IN));
        }
        for outgoing in &self.outgoing {
            fds.push(PollFd::new(&outgoing.pipe, PollFlags::OUT));
        }
    }
}

impl PrimarySelectionDeviceHandler for OverlayState {
    /// 【主选择区】【选择变更】接受 SCTK 已更新的 device 来源，连接及队列来自回调，无返回值，不主动读取正文
    fn selection(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &ZwpPrimarySelectionDeviceV1,
    ) {
    }
}

impl PrimarySelectionSourceHandler for OverlayState {
    /// 【主选择区】【来源发送】按 source、mime 与 write_pipe 排队发送已拥有正文，连接与队列来自回调，无返回值
    fn send_request(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        source: &ZwpPrimarySelectionSourceV1,
        mime: String,
        write_pipe: WritePipe,
    ) {
        if !MIMES.contains(&mime.as_str())
            || self.primary_selection.outgoing.len() >= transfer::MAX_TRANSFERS
        {
            return;
        }
        if let Some(owned) = self
            .primary_selection
            .owned
            .iter()
            .find(|owned| owned.source.inner() == source)
            && let Ok(outgoing) = transfer::Outgoing::new(write_pipe, owned.text.clone())
        {
            self.primary_selection.outgoing.push(outgoing);
        }
    }

    /// 【主选择区】【来源取消】按 source 释放已失效来源与缓存，连接及队列来自回调，无返回值，不清除其他座席选择
    fn cancelled(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        source: &ZwpPrimarySelectionSourceV1,
    ) {
        self.primary_selection
            .owned
            .retain(|owned| owned.source.inner() != source);
    }
}
