//! 【原生窗口】【运行时】共享内存双缓冲与无空转的事件循环

use smithay_client_toolkit::reexports::{
    calloop::{EventLoop, LoopHandle},
    calloop_wayland_source::WaylandSource,
};
use smithay_client_toolkit::{
    compositor::{CompositorState, FrameCallbackData},
    output::OutputState,
    registry::RegistryState,
    seat::{SeatState, pointer::cursor_shape::CursorShapeManager},
    shell::{
        WaylandSurface,
        xdg::{
            XdgShell,
            window::{Window, WindowDecorations},
        },
    },
    shm::{
        Shm,
        slot::{Buffer, SlotPool},
    },
};
use tiny_skia::Pixmap;
use wayland_client::{
    Connection, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_pointer, wl_seat, wl_shm},
};
use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::WpCursorShapeDeviceV1;
use wayland_protocols::{
    wp::{
        fractional_scale::v1::client::{wp_fractional_scale_manager_v1, wp_fractional_scale_v1},
        viewporter::client::{wp_viewport, wp_viewporter},
    },
    xdg::shell::client::xdg_toplevel,
};

use super::{WindowAction, WindowContent, WindowEvent, WindowSpec};
use crate::backend::wayland::text_input::TextInput;
use crate::features::text::input_method::focus::Target;
use crate::{AppResult, keys::Mods};

pub(super) struct NativeWindow {
    pub registry: RegistryState,
    pub outputs: OutputState,
    pub seats: SeatState,
    pub cursor_shapes: Option<CursorShapeManager>,
    pub cursor: Option<WpCursorShapeDeviceV1>,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub pointer_seat: Option<wl_seat::WlSeat>,
    pub keyboard_seat: Option<wl_seat::WlSeat>,
    pub text_input: TextInput,
    pub input_target: Target,
    pub enter_serial: u32,
    pub modifiers: Mods,
    pub shm: Shm,
    pub pool: SlotPool,
    pub buffers: Vec<Buffer>,
    pub canvas: Option<Pixmap>,
    pub window: Window,
    pub viewport: Option<wp_viewport::WpViewport>,
    pub fractional: Option<wp_fractional_scale_v1::WpFractionalScaleV1>,
    pub content: Box<dyn WindowContent>,
    pub size: (u32, u32),
    pub scale: f32,
    pub dirty: bool,
    pub configured: bool,
    pub mapped: bool,
    pub frame_pending: bool,
    pub exit: bool,
    pub loop_handle: LoopHandle<'static, Self>,
}

impl NativeWindow {
    /// 【原生窗口】【事件处理】把 event 交给内容并执行返回动作；serial 仅用于有效鼠标按下请求
    pub fn dispatch(&mut self, event: WindowEvent, serial: Option<u32>) {
        match self.content.event(event) {
            WindowAction::None => {}
            WindowAction::Redraw => self.dirty = true,
            WindowAction::Close => self.exit = true,
            WindowAction::Move | WindowAction::Resize if serial.is_none() => {}
            action => {
                if let (Some(seat), Some(serial)) = (&self.pointer_seat, serial) {
                    if action == WindowAction::Move {
                        self.window.move_(seat, serial);
                    } else {
                        self.window
                            .resize(seat, serial, xdg_toplevel::ResizeEdge::BottomRight);
                    }
                }
            }
        }
        self.reconcile_text_input();
    }

    /// 【原生窗口】【帧提交】按当前大小绘制一帧并请求帧回调；缓冲未释放时等待后续事件
    fn render(&mut self, qh: &QueueHandle<Self>) -> AppResult {
        let width = (self.size.0 as f32 * self.scale).ceil() as i32;
        let height = (self.size.1 as f32 * self.scale).ceil() as i32;
        let pixels = i64::from(width) * i64::from(height);
        if width <= 0 || height <= 0 || pixels > 64 * 1024 * 1024 {
            return Err("window buffer dimensions exceed the supported limit".into());
        }
        let fresh = self.canvas.as_ref().is_none_or(|canvas| {
            canvas.width() != width as u32 || canvas.height() != height as u32
        });
        if fresh {
            self.canvas = Pixmap::new(width as u32, height as u32);
            self.buffers
                .retain(|buffer| buffer.height() == height && buffer.stride() == width * 4);
        }
        let canvas = self
            .canvas
            .as_mut()
            .ok_or("cannot allocate a window canvas")?;
        self.content.draw(canvas, self.scale);

        let available = self
            .buffers
            .iter()
            .position(|buffer| buffer.canvas(&mut self.pool).is_some());
        let index = if let Some(index) = available {
            index
        } else {
            if self.buffers.len() >= 3 {
                return Ok(());
            }
            let (buffer, _) =
                self.pool
                    .create_buffer(width, height, width * 4, wl_shm::Format::Argb8888)?;
            self.buffers.push(buffer);
            self.buffers.len() - 1
        };
        let buffer = &self.buffers[index];
        let bytes = buffer
            .canvas(&mut self.pool)
            .ok_or("window buffer is still active")?;
        crate::utils::copy_swizzled(bytes, canvas.data());
        buffer.attach_to(self.window.wl_surface())?;
        self.window.wl_surface().damage_buffer(0, 0, width, height);
        self.window
            .wl_surface()
            .frame(qh, FrameCallbackData(self.window.wl_surface().clone()));
        self.window.commit();
        if !self.mapped {
            self.content.mapped();
            self.mapped = true;
        }
        self.frame_pending = true;
        self.dirty = false;
        Ok(())
    }
}

/// 【原生窗口】【启动】创建 spec 指定的普通窗口并运行 content；空闲时阻塞，关闭时释放全部窗口资源
pub fn run(spec: WindowSpec, mut content: impl WindowContent + 'static) -> AppResult {
    let connection = Connection::connect_to_env()?;
    let (globals, queue) = registry_queue_init(&connection)?;
    let qh = queue.handle();
    let mut event_loop: EventLoop<NativeWindow> = EventLoop::try_new()?;
    WaylandSource::new(connection.clone(), queue)
        .insert(event_loop.handle())
        .map_err(|error| error.error)?;
    let (wake, source) = super::wake::source()?;
    event_loop
        .handle()
        .insert_source(source, |(), &mut (), state| {
            state.dispatch(WindowEvent::Wake, None);
        })
        .map_err(|error| error.error)?;
    content.connected(wake);
    let compositor = CompositorState::bind(&globals, &qh)?;
    let shell = XdgShell::bind(&globals, &qh)?;
    let shm = Shm::bind(&globals, &qh)?;
    let pool = SlotPool::new(4096, &shm)?;
    let viewporter = globals
        .bind::<wp_viewporter::WpViewporter, _, _>(&qh, 1..=1, ())
        .ok();
    let fractional_manager = globals
        .bind::<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1, _, _>(&qh, 1..=1, ())
        .ok();
    let window = shell.create_window(
        compositor.create_surface(&qh),
        WindowDecorations::ClientOnly,
        &qh,
    );
    window.set_title(spec.title);
    window.set_app_id(spec.app_id);
    window.set_min_size(Some(spec.minimum));
    let viewport = viewporter.map(|manager| manager.get_viewport(window.wl_surface(), &qh, ()));
    let fractional = viewport.as_ref().and_then(|_| {
        fractional_manager.map(|manager| manager.get_fractional_scale(window.wl_surface(), &qh, ()))
    });
    window.commit();
    let mut text_input = TextInput::default();
    text_input.manager = globals
        .bind::<wayland_protocols::wp::text_input::zv3::client::zwp_text_input_manager_v3::ZwpTextInputManagerV3, _, _>(&qh, 1..=1, ())
        .ok();

    let mut state = NativeWindow {
        registry: RegistryState::new(&globals),
        outputs: OutputState::new(&globals, &qh),
        seats: SeatState::new(&globals, &qh),
        cursor_shapes: CursorShapeManager::bind(&globals, &qh).ok(),
        cursor: None,
        pointer: None,
        keyboard: None,
        pointer_seat: None,
        keyboard_seat: None,
        text_input,
        input_target: Target::default(),
        enter_serial: 0,
        modifiers: Mods::default(),
        shm,
        pool,
        buffers: Vec::new(),
        canvas: None,
        window,
        viewport,
        fractional,
        content: Box::new(content),
        size: spec.size,
        scale: 1.0,
        dirty: true,
        configured: false,
        mapped: false,
        frame_pending: false,
        exit: false,
        loop_handle: event_loop.handle(),
    };
    while !state.exit {
        if state.configured && state.dirty && !state.frame_pending {
            state.render(&qh)?;
        }
        state.sync_text_input();
        connection.flush()?;
        event_loop.dispatch(None, &mut state)?;
    }
    // 1. 【原生窗口】【关闭交接】嵌套窗口不会退出进程，先析构表面再发送销毁请求
    drop(state);
    connection.flush()?;
    Ok(())
}
