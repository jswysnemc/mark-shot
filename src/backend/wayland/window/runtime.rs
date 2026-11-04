//! 【原生窗口】【运行时】同连接管理父子窗口，共享分配器且保持无空转事件循环

use smithay_client_toolkit::reexports::{
    calloop::{EventLoop, LoopHandle},
    calloop_wayland_source::WaylandSource,
};
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    seat::{SeatState, pointer::cursor_shape::CursorShapeManager},
    shell::xdg::XdgShell,
    shm::{Shm, slot::SlotPool},
};
use wayland_client::{
    Connection,
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_pointer, wl_seat},
};
use wayland_protocols::{
    wp::{
        cursor_shape::v1::client::wp_cursor_shape_device_v1::WpCursorShapeDeviceV1,
        fractional_scale::v1::client::wp_fractional_scale_manager_v1,
        viewporter::client::wp_viewporter,
    },
    xdg::{dialog::v1::client::xdg_wm_dialog_v1, shell::client::xdg_toplevel},
};

use super::{
    WindowAction, WindowContent, WindowEvent, WindowSpec, WindowWake,
    dialog::Dialog,
    surface::{Surface, SurfaceId},
};
use crate::{
    AppResult, backend::wayland::text_input::TextInput,
    features::text::input_method::focus::Target, keys::Mods,
};

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
    pub keyboard_surface: Option<SurfaceId>,
    pub text_surface: Option<SurfaceId>,
    pub text_input: TextInput,
    pub input_target: Target,
    pub enter_serial: u32,
    pub modifiers: Mods,
    pub compositor: CompositorState,
    pub shell: XdgShell,
    pub viewporter: Option<wp_viewporter::WpViewporter>,
    pub fractional_manager: Option<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1>,
    pub dialog_manager: Option<xdg_wm_dialog_v1::XdgWmDialogV1>,
    pub shm: Shm,
    pub pool: SlotPool,
    pub main: Surface,
    pub dialog: Option<Dialog>,
    pub content: Box<dyn WindowContent>,
    pub wake: WindowWake,
    pub exit: bool,
    pub loop_handle: LoopHandle<'static, Self>,
}

impl NativeWindow {
    /// 【原生窗口】【父窗口事件】将 event 路由到父内容，serial 仅用于有效鼠标按下操作，无返回值
    pub fn dispatch(&mut self, event: WindowEvent, serial: Option<u32>) {
        self.dispatch_to(SurfaceId::Main, event, serial);
    }

    /// 【原生窗口】【独立事件】按 id 路由 event 和 serial，模态时过滤父窗口输入，过期身份不影响新窗口，无返回值
    pub fn dispatch_to(&mut self, id: SurfaceId, event: WindowEvent, serial: Option<u32>) {
        if id == SurfaceId::Main && self.dialog.is_some() && event.is_interaction() {
            return;
        }
        let action = match id {
            SurfaceId::Main => self.content.event(event),
            SurfaceId::Dialog(id) => {
                let Some(dialog) = self.dialog.as_mut().filter(|dialog| dialog.id == id) else {
                    return;
                };
                dialog.content.event(event)
            }
        };
        self.apply(id, action, serial);
        self.reconcile_text_input();
    }

    /// 【原生窗口】【动作执行】执行 id 内容返回的 action，关闭子窗口只回传结果；serial 限制移动和调整请求，无返回值
    pub fn apply(&mut self, id: SurfaceId, action: WindowAction, serial: Option<u32>) {
        match action {
            WindowAction::None => {}
            WindowAction::Redraw => {
                if let Some(surface) = self.surface_mut(id) {
                    surface.dirty = true;
                }
            }
            WindowAction::Close => match id {
                SurfaceId::Main => self.exit = true,
                SurfaceId::Dialog(id) => {
                    let accepted = self
                        .dialog
                        .as_ref()
                        .filter(|dialog| dialog.id == id)
                        .is_some_and(|dialog| dialog.content.dialog_accepted());
                    self.finish_dialog(id, accepted);
                }
            },
            WindowAction::Move | WindowAction::Resize => {
                if let (Some(seat), Some(serial), Some(surface)) =
                    (&self.pointer_seat, serial, self.surface(id))
                {
                    if action == WindowAction::Move {
                        surface.window.move_(seat, serial);
                    } else {
                        surface
                            .window
                            .resize(seat, serial, xdg_toplevel::ResizeEdge::BottomRight);
                    }
                }
            }
        }
    }

    /// 【原生窗口】【只读表面】按 id 查询存活表面，过期子窗口返回空值，不回退到父窗口
    pub fn surface(&self, id: SurfaceId) -> Option<&Surface> {
        match id {
            SurfaceId::Main => Some(&self.main),
            SurfaceId::Dialog(id) => self
                .dialog
                .as_ref()
                .filter(|dialog| dialog.id == id)
                .map(|dialog| &dialog.surface),
        }
    }
}

/// 【原生窗口】【启动】创建 spec 指定的父窗口并运行 content，子窗口保持同连接；返回连接或提交错误，空闲阻塞
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
            if let Some(id) = state.dialog.as_ref().map(|dialog| dialog.id) {
                state.dispatch_to(SurfaceId::Dialog(id), WindowEvent::Wake, None);
            }
        })
        .map_err(|error| error.error)?;
    content.connected(wake.clone());
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
    let dialog_manager = globals
        .bind::<xdg_wm_dialog_v1::XdgWmDialogV1, _, _>(&qh, 1..=1, ())
        .ok();
    let main = Surface::new(
        spec,
        SurfaceId::Main,
        &compositor,
        &shell,
        viewporter.as_ref(),
        fractional_manager.as_ref(),
        &qh,
    );
    use smithay_client_toolkit::shell::WaylandSurface;
    main.window.commit();
    let mut text_input = TextInput::default();
    text_input.manager = globals.bind::<wayland_protocols::wp::text_input::zv3::client::zwp_text_input_manager_v3::ZwpTextInputManagerV3, _, _>(&qh, 1..=1, ()).ok();
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
        keyboard_surface: None,
        text_surface: None,
        text_input,
        input_target: Target::default(),
        enter_serial: 0,
        modifiers: Mods::default(),
        compositor,
        shell,
        viewporter,
        fractional_manager,
        dialog_manager,
        shm,
        pool,
        main,
        dialog: None,
        content: Box::new(content),
        wake,
        exit: false,
        loop_handle: event_loop.handle(),
    };
    while !state.exit {
        state.sync_dialog(&qh);
        state
            .main
            .render(state.content.as_mut(), &mut state.pool, &qh)?;
        if let Some(dialog) = &mut state.dialog {
            dialog
                .surface
                .render(dialog.content.as_mut(), &mut state.pool, &qh)?;
        }
        state.sync_text_input();
        connection.flush()?;
        event_loop.dispatch(None, &mut state)?;
    }
    // 1. 【原生窗口】【关闭交接】先关闭所属子窗口，再析构父表面，最后发送销毁请求
    state.dialog.take();
    drop(state);
    connection.flush()?;
    Ok(())
}
