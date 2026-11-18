//! 【共享内存测试】【私有协议服务】通过独立套接字控制真实缓冲释放，不连接桌面或图形驱动

mod shell;

use std::{
    fs::File,
    os::{fd::AsRawFd, unix::net::UnixStream},
    sync::{Arc, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};
use wayland_server::{
    Client, DataInit, Dispatch, Display, DisplayHandle, GlobalDispatch, New, Resource,
    protocol::{wl_buffer, wl_compositor, wl_region, wl_shm, wl_shm_pool, wl_surface},
};

#[derive(Clone, Debug, Default)]
pub(crate) struct Snapshot {
    pub pool_seals: Vec<i32>,
    pub pool_sizes: Vec<i32>,
    pub buffer_offsets: Vec<i32>,
    pub destroyed_buffers: Vec<usize>,
    pub attached_buffers: Vec<usize>,
}

#[derive(Default)]
struct State {
    snapshot: Snapshot,
    files: Vec<File>,
    buffers: Vec<wl_buffer::WlBuffer>,
    shells: shell::Shells,
}

enum Command {
    Snapshot(mpsc::Sender<Snapshot>),
    Release(usize),
    ConfigureWindow(i32, i32),
    ConfigureLayer,
    Stop,
}

pub(crate) struct Server {
    commands: mpsc::Sender<Command>,
    worker: Option<JoinHandle<()>>,
}

impl Server {
    /// 【共享内存测试】【服务创建】无参数，返回仅相互连接的私有服务和客户端连接
    pub(crate) fn new() -> (Self, wayland_client::Connection) {
        let (client_socket, server_socket) = UnixStream::pair().unwrap();
        client_socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let (commands, receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut display = Display::<State>::new().unwrap();
            let mut handle = display.handle();
            handle.insert_client(server_socket, Arc::new(())).unwrap();
            handle.create_global::<State, wl_shm::WlShm, _>(1, ());
            handle.create_global::<State, wl_compositor::WlCompositor, _>(6, ());
            shell::globals(&handle);
            let mut state = State::default();
            loop {
                // 1. 【共享内存测试】【请求分派】分派客户端请求后处理测试指令，释放时机只由测试决定
                display.dispatch_clients(&mut state).unwrap();
                display.flush_clients().unwrap();
                match receiver.recv_timeout(Duration::from_millis(2)) {
                    Ok(Command::Snapshot(reply)) => {
                        reply.send(state.snapshot.clone()).unwrap();
                    }
                    Ok(Command::Release(index)) => state.buffers[index].release(),
                    Ok(Command::ConfigureWindow(width, height)) => {
                        state.shells.configure_window(width, height);
                    }
                    Ok(Command::ConfigureLayer) => state.shells.configure_layer(),
                    Ok(Command::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                display.flush_clients().unwrap();
            }
        });
        let connection = wayland_client::Connection::from_socket(client_socket).unwrap();
        (
            Self {
                commands,
                worker: Some(worker),
            },
            connection,
        )
    }

    /// 【共享内存测试】【请求快照】无参数，返回服务端已经分派的池、附着与销毁记录
    pub(crate) fn snapshot(&self) -> Snapshot {
        let (sender, receiver) = mpsc::channel();
        self.commands.send(Command::Snapshot(sender)).unwrap();
        receiver.recv_timeout(Duration::from_secs(5)).unwrap()
    }

    /// 【共享内存测试】【缓冲释放】按创建顺序 index 发送真实 wl_buffer.release，无返回值
    pub(crate) fn release(&self, index: usize) {
        self.commands.send(Command::Release(index)).unwrap();
        // 1. 【共享内存测试】【事件同步】等待指令已经执行，使后续客户端往返必定读到释放事件
        self.snapshot();
    }

    /// 【共享内存测试】【窗口配置】按 width 和 height 发出完整 xdg 配置批次，无返回值
    pub(crate) fn configure_window(&self, width: i32, height: i32) {
        self.commands
            .send(Command::ConfigureWindow(width, height))
            .unwrap();
        self.snapshot();
    }

    /// 【共享内存测试】【探测配置】无参数，发出真实 layer 配置事件，无返回值
    pub(crate) fn configure_layer(&self) {
        self.commands.send(Command::ConfigureLayer).unwrap();
        self.snapshot();
    }
}

impl Drop for Server {
    /// 【共享内存测试】【服务退出】无参数，结束线程并关闭私有连接，无返回值
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Stop);
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}

macro_rules! global {
    ($interface:ty) => {
        impl GlobalDispatch<$interface, ()> for State {
            /// 【共享内存测试】【全局绑定】按 resource 初始化协议全局，其他上下文不改变状态，无返回值
            fn bind(
                _: &mut Self,
                _: &DisplayHandle,
                _: &Client,
                resource: New<$interface>,
                _: &(),
                data: &mut DataInit<'_, Self>,
            ) {
                data.init(resource, ());
            }
        }
    };
}
pub(super) use global;

impl GlobalDispatch<wl_shm::WlShm, ()> for State {
    /// 【共享内存测试】【格式声明】按 resource 初始化共享全局并声明常用像素格式，无返回值
    fn bind(
        _: &mut Self,
        _: &DisplayHandle,
        _: &Client,
        resource: New<wl_shm::WlShm>,
        _: &(),
        data: &mut DataInit<'_, Self>,
    ) {
        let resource = data.init(resource, ());
        resource.format(wl_shm::Format::Argb8888);
        resource.format(wl_shm::Format::Xrgb8888);
    }
}

global!(wl_compositor::WlCompositor);

impl Dispatch<wl_shm::WlShm, ()> for State {
    /// 【共享内存测试】【池接收】从 request 保存服务端收到的文件、密封值与容量，无返回值
    fn request(
        state: &mut Self,
        _: &Client,
        _: &wl_shm::WlShm,
        request: wl_shm::Request,
        _: &(),
        _: &DisplayHandle,
        data: &mut DataInit<'_, Self>,
    ) {
        if let wl_shm::Request::CreatePool { id, fd, size } = request {
            let index = state.files.len();
            let seals = unsafe { nix::libc::fcntl(fd.as_raw_fd(), nix::libc::F_GET_SEALS) };
            state.snapshot.pool_seals.push(seals);
            state.snapshot.pool_sizes.push(size);
            state.files.push(File::from(fd));
            data.init(id, index);
        }
    }
}

impl Dispatch<wl_shm_pool::WlShmPool, usize> for State {
    /// 【共享内存测试】【缓冲创建】按 request 和池 index 记录真实缓冲偏移或扩容，无返回值
    fn request(
        state: &mut Self,
        _: &Client,
        _: &wl_shm_pool::WlShmPool,
        request: wl_shm_pool::Request,
        index: &usize,
        _: &DisplayHandle,
        data: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_shm_pool::Request::CreateBuffer { id, offset, .. } => {
                let index = state.buffers.len();
                state.buffers.push(data.init(id, index));
                state.snapshot.buffer_offsets.push(offset);
            }
            wl_shm_pool::Request::Resize { size } => state.snapshot.pool_sizes[*index] = size,
            _ => {}
        }
    }
}

impl Dispatch<wl_buffer::WlBuffer, usize> for State {
    /// 【共享内存测试】【销毁记录】按 request 和缓冲 index 记录客户端实际销毁，无返回值
    fn request(
        state: &mut Self,
        _: &Client,
        _: &wl_buffer::WlBuffer,
        request: wl_buffer::Request,
        index: &usize,
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        if let wl_buffer::Request::Destroy = request {
            state.snapshot.destroyed_buffers.push(*index);
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for State {
    /// 【共享内存测试】【表面创建】按 request 初始化表面或输入区域，无返回值
    fn request(
        _: &mut Self,
        _: &Client,
        _: &wl_compositor::WlCompositor,
        request: wl_compositor::Request,
        _: &(),
        _: &DisplayHandle,
        data: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_compositor::Request::CreateSurface { id } => {
                data.init(id, ());
            }
            wl_compositor::Request::CreateRegion { id } => {
                data.init(id, ());
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for State {
    /// 【共享内存测试】【附着记录】从 request 记录真实缓冲附着，帧回调立即完成且不释放缓冲，无返回值
    fn request(
        state: &mut Self,
        _: &Client,
        _: &wl_surface::WlSurface,
        request: wl_surface::Request,
        _: &(),
        _: &DisplayHandle,
        data: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_surface::Request::Attach {
                buffer: Some(buffer),
                ..
            } => {
                state
                    .snapshot
                    .attached_buffers
                    .push(*buffer.data::<usize>().unwrap());
            }
            wl_surface::Request::Frame { callback } => {
                data.init(callback, ()).done(0);
            }
            _ => {}
        }
    }
}

macro_rules! noop {
    ($interface:ty) => {
        impl Dispatch<$interface, ()> for State {
            /// 【共享内存测试】【无状态请求】接收不创建对象的 request，其他参数仅满足协议接口，无返回值
            fn request(
                _: &mut Self,
                _: &Client,
                _: &$interface,
                _: <$interface as Resource>::Request,
                _: &(),
                _: &DisplayHandle,
                _: &mut DataInit<'_, Self>,
            ) {
            }
        }
    };
}
pub(super) use noop;
noop!(wl_region::WlRegion);
noop!(wayland_server::protocol::wl_callback::WlCallback);
