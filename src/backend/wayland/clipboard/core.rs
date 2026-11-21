//! 【Wayland剪贴板】【核心协议】以当前窗口焦点和真实输入序列拥有剪贴板，取消后回收数据

use super::{
    context::{self, Content},
    payload::Payload,
    transfer::{self, Outgoing},
};
use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    time::{Duration, Instant},
};
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    globals::GlobalList,
    protocol::{
        wl_data_device::WlDataDevice, wl_data_device_manager::WlDataDeviceManager,
        wl_data_source::WlDataSource, wl_seat::WlSeat,
    },
};

struct Owned {
    source: WlDataSource,
    payload: Payload,
}

pub(crate) struct Core {
    manager: WlDataDeviceManager,
    devices: Vec<(WlSeat, WlDataDevice)>,
    inputs: VecDeque<(Option<u32>, WlSeat, u32)>,
    current: Option<(WlSeat, u32)>,
    focused: Vec<WlSeat>,
    owned: Option<Owned>,
    outgoing: Vec<Outgoing>,
}

impl Core {
    /// 【Wayland剪贴板】【协议选择】按 globals 和 qh 绑定核心协议，返回数据控制缺失时的状态；原生会话返回空值
    pub fn bind<D>(globals: &GlobalList, qh: &QueueHandle<D>) -> Option<Rc<RefCell<Self>>>
    where
        D: Dispatch<WlDataDeviceManager, ()> + 'static,
    {
        let controlled = globals.contents().with_list(|globals| {
            globals.iter().any(|global| {
                matches!(
                    global.interface.as_str(),
                    "ext_data_control_manager_v1" | "zwlr_data_control_manager_v1"
                )
            })
        });
        if controlled {
            return None;
        }
        let manager = globals.bind(qh, 1..=3, ()).ok()?;
        Some(Rc::new(RefCell::new(Self {
            manager,
            devices: Vec::new(),
            inputs: VecDeque::new(),
            current: None,
            focused: Vec::new(),
            owned: None,
            outgoing: Vec::new(),
        })))
    }

    /// 【Wayland剪贴板】【设备绑定】按 seat 和 qh 创建唯一剪贴板设备，无返回值
    pub fn seat<D>(&mut self, seat: &WlSeat, qh: &QueueHandle<D>)
    where
        D: Dispatch<WlDataDevice, ()> + 'static,
    {
        if !self.devices.iter().any(|(owner, _)| owner == seat) {
            self.devices
                .push((seat.clone(), self.manager.get_data_device(seat, qh, ())));
        }
    }

    /// 【Wayland剪贴板】【窗口范围】按 core、qh 和 connection 注册同步复制处理器，返回自动恢复句柄
    pub fn scope<D>(
        core: &Rc<RefCell<Self>>,
        qh: QueueHandle<D>,
        connection: Connection,
    ) -> context::Scope
    where
        D: Dispatch<WlDataSource, ()> + 'static,
    {
        let core = core.clone();
        context::enter(move |content| {
            core.borrow_mut().set(content, &qh)?;
            connection.flush()?;
            Ok(())
        })
    }

    /// 【Wayland剪贴板】【原始输入】按 seat、serial 和键盘身份 key 排队授权，无返回值，避免未来输入覆盖当前操作
    pub fn input(&mut self, seat: WlSeat, serial: u32, key: bool) {
        self.inputs.push_back((key.then_some(serial), seat, serial));
    }

    /// 【Wayland剪贴板】【输入交付】按 keyboard_serial 匹配当前应用事件，无返回值；鼠标释放也使用真实序列
    pub fn advance(&mut self, keyboard_serial: Option<u32>) {
        if let Some(index) = self
            .inputs
            .iter()
            .position(|(key, _, _)| *key == keyboard_serial)
            && let Some((_, seat, serial)) = self.inputs.remove(index)
        {
            self.current = Some((seat, serial));
        }
    }

    /// 【Wayland剪贴板】【焦点更新】按 seat 和 focused 记录真实键盘焦点，无返回值，失焦后禁止设置选择
    pub fn focus(&mut self, seat: WlSeat, focused: bool) {
        self.focused.retain(|owner| owner != &seat);
        if focused {
            self.focused.push(seat);
        }
    }

    /// 【Wayland剪贴板】【初始焦点】按 seat 和 serial 保存合成器真实进入事件，返回空值，不覆盖已有输入操作
    pub fn enter(&mut self, seat: WlSeat, serial: u32) {
        self.focus(seat.clone(), true);
        if self.current.is_none() {
            self.current = Some((seat, serial));
        }
    }

    /// 【Wayland剪贴板】【内容拥有】按 content 和 qh 设置当前授权座席的选择，返回结果，缺少焦点或输入授权时明确失败
    fn set<D>(&mut self, content: Content, qh: &QueueHandle<D>) -> crate::AppResult
    where
        D: Dispatch<WlDataSource, ()> + 'static,
    {
        // 1. 【Wayland剪贴板】【授权验证】当前操作必须来自仍有键盘焦点的有效座席
        let (seat, serial) = self
            .current
            .as_ref()
            .ok_or("clipboard copy requires an input event from the focused window")?;
        if !self.focused.contains(seat) {
            return Err("clipboard window no longer has keyboard focus".into());
        }
        let device = self
            .devices
            .iter()
            .find(|(owner, _)| owner == seat)
            .map(|(_, device)| device)
            .ok_or("clipboard seat is unavailable")?;
        // 2. 【Wayland剪贴板】【格式登记】一次编码内容并登记兼容类型
        let payload = Payload::new(content);
        let source = self.manager.create_data_source(qh, ());
        for mime in payload.mimes() {
            source.offer((*mime).into());
        }
        // 3. 【Wayland剪贴板】【来源交接】提交当前选择后释放旧来源，已开始的传输独立完成
        device.set_selection(Some(&source), *serial);
        if let Some(previous) = self.owned.replace(Owned { source, payload }) {
            previous.source.destroy();
        }
        log::info!("【Wayland剪贴板】【核心复制】已使用窗口输入序列 {serial} 设置选择");
        Ok(())
    }

    /// 【Wayland剪贴板】【来源发送】按 source、mime 和 fd 排队有限非阻塞传输，无返回值，未知来源或格式不发送数据
    pub fn send(&mut self, source: &WlDataSource, mime: &str, fd: std::os::fd::OwnedFd) {
        if self.outgoing.len() >= transfer::MAX_TRANSFERS {
            return;
        }
        let Some(owned) = self.owned.as_ref().filter(|owned| &owned.source == source) else {
            return;
        };
        let result = owned.payload.bytes(mime).and_then(|bytes| {
            bytes
                .map(|bytes| Outgoing::new(fd.into(), bytes))
                .transpose()
                .map_err(Into::into)
        });
        match result {
            Ok(Some(outgoing)) => self.outgoing.push(outgoing),
            Ok(None) => {}
            Err(error) => log::error!("【Wayland剪贴板】【传输准备】{error}"),
        }
    }

    /// 【Wayland剪贴板】【拥有取消】按 source 释放被接管的选择，无返回值，已开始传输继续完成
    pub fn cancelled(&mut self, source: &WlDataSource) {
        if self
            .owned
            .as_ref()
            .is_some_and(|owned| &owned.source == source)
            && let Some(owned) = self.owned.take()
        {
            owned.source.destroy();
        }
    }

    /// 【Wayland剪贴板】【活动判断】无参数，返回是否仍拥有选择或处理传输
    pub fn active(&self) -> bool {
        self.owned.is_some() || !self.outgoing.is_empty()
    }
    /// 【Wayland剪贴板】【数据推进】无参数，推进当前可写管道并回收完成传输，无返回值
    pub fn progress(&mut self) {
        self.outgoing.retain_mut(|outgoing| !outgoing.progress());
    }
    /// 【Wayland剪贴板】【传输期限】无参数，返回最早传输剩余时间，空闲持有选择时无限等待
    pub fn timeout(&self) -> Option<Duration> {
        self.outgoing
            .iter()
            .map(|item| item.deadline)
            .min()
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
    }
    /// 【Wayland剪贴板】【描述符登记】按 fds 登记当前管道写入兴趣，无返回值
    pub fn poll_fds<'a>(&'a self, fds: &mut Vec<rustix::event::PollFd<'a>>) {
        for outgoing in &self.outgoing {
            fds.push(rustix::event::PollFd::new(
                &outgoing.pipe,
                rustix::event::PollFlags::OUT,
            ));
        }
    }
}

impl Drop for Core {
    /// 【Wayland剪贴板】【资源释放】无参数，销毁本连接的数据源、设备和管理器，无返回值
    fn drop(&mut self) {
        if let Some(owned) = self.owned.take() {
            owned.source.destroy();
        }
        for (_, device) in &self.devices {
            if wayland_client::Proxy::version(device) >= 2 {
                device.release();
            }
        }
    }
}
