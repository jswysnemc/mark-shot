//! 【贴图】【窗口运行时】共享内存缓冲与事件驱动循环，置顶使用 layer-shell，普通窗口使用 xdg

use super::popup::PinPopup;
use super::{gesture::Gesture, preview::Preview};
use crate::{
    AppResult,
    features::pinned_image::{
        model::PinnedImage,
        placement::{Geometry, Placement},
    },
    keys::Mods,
    ocr::{OcrText, OcrView},
};
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::{
        calloop::{
            EventLoop,
            channel::{self, Sender},
        },
        calloop_wayland_source::WaylandSource,
    },
    registry::RegistryState,
    seat::{SeatState, pointer::cursor_shape::CursorShapeManager},
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface},
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
use std::sync::{Arc, atomic::AtomicBool};
use wayland_client::{
    Connection, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_surface},
};
use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1, wp_fractional_scale_v1,
};
use wayland_protocols::wp::{
    cursor_shape::v1::client::wp_cursor_shape_device_v1::WpCursorShapeDeviceV1,
    viewporter::client::{wp_viewport, wp_viewporter},
};

pub(super) enum Shell {
    Window(Window),
    Layer(LayerSurface),
}
impl Shell {
    /// 【贴图】【协议表面】返回当前窗口角色对应的表面，不新建协议对象
    pub fn surface(&self) -> &wl_surface::WlSurface {
        match self {
            Self::Window(window) => window.wl_surface(),
            Self::Layer(layer) => layer.wl_surface(),
        }
    }
}

pub(super) struct Pin {
    pub connection: Connection,
    pub registry: RegistryState,
    pub outputs: OutputState,
    pub seats: SeatState,
    pub compositor: CompositorState,
    pub xdg: XdgShell,
    pub layers: Option<LayerShell>,
    pub shell: Shell,
    pub viewporter: Option<wp_viewporter::WpViewporter>,
    pub viewport: Option<wp_viewport::WpViewport>,
    pub fractional_manager: Option<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1>,
    pub fractional: Option<wp_fractional_scale_v1::WpFractionalScaleV1>,
    pub shm: Shm,
    pub pool: SlotPool,
    pub buffers: Vec<Buffer>,
    pub image: PinnedImage,
    pub output: Option<wl_output::WlOutput>,
    pub bounds: (u32, u32),
    pub position: (i32, i32),
    pub initial_position: Option<(i32, i32)>,
    pub initialized: bool,
    pub placement: Placement,
    pub size: (u32, u32),
    pub render_scale: f32,
    pub configured: bool,
    pub dirty: bool,
    pub frame_pending: bool,
    pub exit: bool,
    pub pointer_seat: Option<wl_seat::WlSeat>,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub cursor_shapes: Option<CursorShapeManager>,
    pub cursor: Option<WpCursorShapeDeviceV1>,
    pub serial: u32,
    pub mods: Mods,
    pub drag: Option<Gesture>,
    pub native_resizing: bool,
    pub preview: Option<Preview>,
    pub clicks: crate::interaction::DoubleClickTracker<bool>,
    pub popup: Option<PinPopup>,
    pub font_system: cosmic_text::FontSystem,
    pub swash: cosmic_text::SwashCache,
    pub icons: crate::renderer::Icons,
    pub text: OcrView,
    pub text_selection: bool,
    pub selecting: bool,
    pub generation: u64,
    pub ocr_cancel: Option<Arc<AtomicBool>>,
    pub ocr_sender: Sender<(u64, Result<OcrText, String>)>,
    pub copy_after_ocr: bool,
    pub translation: crate::features::pinned_image::translation::Translation,
    pub translation_task: Option<crate::translation::task::Task>,
    pub translation_retired: Vec<crate::translation::task::Task>,
    pub feedback: crate::features::pinned_image::feedback::Feedback,
    pub translation_wake: smithay_client_toolkit::reexports::calloop::ping::Ping,
    pub last_position: (f64, f64),
}

impl Pin {
    /// 【贴图】【角色重建】按置顶设置重建表面，原图和当前缩放保持不变；qh 用于绑定新协议对象
    pub fn recreate(&mut self, qh: &QueueHandle<Self>) {
        self.popup = None;
        if !self.image.always_on_top {
            self.placement = Placement {
                logical: self.geometry(),
                visible: self.geometry(),
                offset: (0, 0),
                margin: (0, 0),
            };
        }
        if let Some(fractional) = self.fractional.take() {
            fractional.destroy();
        }
        if let Some(viewport) = self.viewport.take() {
            viewport.destroy()
        }
        self.shell = create_shell(
            &self.compositor,
            &self.xdg,
            self.layers.as_ref(),
            self.image.always_on_top,
            self.output.as_ref(),
            (self.placement.visible.width, self.placement.visible.height),
            self.placement.margin,
            qh,
        );
        self.viewport = self
            .viewporter
            .as_ref()
            .map(|manager| manager.get_viewport(self.shell.surface(), qh, ()));
        self.fractional = self.viewport.as_ref().and_then(|_| {
            self.fractional_manager
                .as_ref()
                .map(|manager| manager.get_fractional_scale(self.shell.surface(), qh, ()))
        });
        self.configured = false;
        self.frame_pending = false;
        self.dirty = true;
        self.shell.surface().commit();
    }
}

impl Drop for Pin {
    /// 【贴图】【任务结束】关闭窗口时通知识别任务取消，不等待模型推理
    fn drop(&mut self) {
        if let Some(cancel) = &self.ocr_cancel {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(mut task) = self.translation_task.take() {
            task.shutdown();
        }
        for mut task in self.translation_retired.drain(..) {
            task.shutdown();
        }
    }
}

/// 【贴图】【窗口角色】创建 size、position 指定的窗口，top 且 layers 可用时采用真正置顶表面
pub(super) fn create_shell(
    compositor: &CompositorState,
    xdg: &XdgShell,
    layers: Option<&LayerShell>,
    top: bool,
    output: Option<&wl_output::WlOutput>,
    size: (u32, u32),
    position: (i32, i32),
    qh: &QueueHandle<Pin>,
) -> Shell {
    let surface = compositor.create_surface(qh);
    if let Some(layers) = layers.filter(|_| top) {
        let layer =
            layers.create_layer_surface(qh, surface, Layer::Top, Some("mark-shot-pin"), output);
        layer.set_anchor(Anchor::TOP | Anchor::LEFT);
        layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        layer.set_exclusive_zone(-1);
        layer.set_size(size.0, size.1);
        layer.set_margin(position.1, 0, 0, position.0);
        Shell::Layer(layer)
    } else {
        let window = xdg.create_window(surface, WindowDecorations::ClientOnly, qh);
        window.set_title("Pinned Mark Shot");
        window.set_app_id("mark-shot-pin");
        window.set_min_size(Some((size.0.min(24), size.1.min(24))));
        Shell::Window(window)
    }
}

/// 【贴图】【显示入口】读取 bytes 的原图并按 density 显示；空闲阻塞，关闭后释放缓冲和连接
pub fn run(bytes: &[u8], density: f32) -> AppResult {
    run_at(bytes, density, None)
}

/// 【贴图】【定位启动】按 initial_position 选择目标输出并恢复选区位置，缺省时在首个输出居中
pub(super) fn run_at(
    bytes: &[u8],
    density: f32,
    initial_position: Option<(i32, i32)>,
) -> AppResult {
    run_at_seed(bytes, density, initial_position, None)
}

/// 【贴图】【译文启动】按 bytes、density、initial_position 和 seed 初始化原图与译文，返回窗口执行结果，已有识别结果不再启动模型
pub(super) fn run_at_seed(
    bytes: &[u8],
    density: f32,
    initial_position: Option<(i32, i32)>,
    seed: Option<crate::features::image_translation::seed::Seed>,
) -> AppResult {
    let connection = match crate::backend::session::Session::connect_to_env()? {
        crate::backend::session::Session::Wayland(connection) => connection,
        crate::backend::session::Session::X11(_) => {
            return crate::features::pinned_image::native_window::run_at_seed(
                bytes,
                density,
                initial_position,
                seed,
            );
        }
    };
    let mut image = PinnedImage::decode(bytes, density)?;
    let (globals, queue) = registry_queue_init(&connection)?;
    let qh = queue.handle();
    let mut event_loop: EventLoop<Pin> = EventLoop::try_new()?;
    super::super::theme_notifications::install(&event_loop.handle(), Pin::refresh_theme)?;
    WaylandSource::new(connection.clone(), queue)
        .insert(event_loop.handle())
        .map_err(|error| error.error)?;
    let (sender, receiver) = channel::channel();
    let (translation_wake, translation_source) =
        smithay_client_toolkit::reexports::calloop::ping::make_ping()?;
    event_loop
        .handle()
        .insert_source(translation_source, |_, _, state| state.poll_translation())
        .map_err(|error| error.error)?;
    event_loop
        .handle()
        .insert_source(receiver, |event, _, state| {
            if let channel::Event::Msg((generation, result)) = event {
                state.receive_ocr(generation, result);
            }
        })
        .map_err(|error| error.error)?;
    let outputs = OutputState::new(&globals, &qh);
    let output = outputs.outputs().next();
    let info = output.as_ref().and_then(|output| outputs.info(output));
    let bounds = info
        .as_ref()
        .and_then(|info| info.logical_size)
        .map(|(w, h)| (w.max(1) as u32, h.max(1) as u32))
        .unwrap_or((1920, 1080));
    image.set_zoom(1.0, (bounds.0 * 9 / 10, bounds.1 * 9 / 10));
    let size = image.size();
    let position = (
        (bounds.0.saturating_sub(size.0) / 2) as i32,
        (bounds.1.saturating_sub(size.1) / 2) as i32,
    );
    let compositor = CompositorState::bind(&globals, &qh)?;
    let xdg = XdgShell::bind(&globals, &qh)?;
    let layers = LayerShell::bind(&globals, &qh).ok();
    image.always_on_top = layers.is_some() && crate::config::get().pinned.always_on_top;
    let shell = create_shell(
        &compositor,
        &xdg,
        layers.as_ref(),
        image.always_on_top,
        output.as_ref(),
        size,
        position,
        &qh,
    );
    let viewporter = globals
        .bind::<wp_viewporter::WpViewporter, _, _>(&qh, 1..=1, ())
        .ok();
    let viewport = viewporter
        .as_ref()
        .map(|manager| manager.get_viewport(shell.surface(), &qh, ()));
    let fractional_manager = globals
        .bind::<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1, _, _>(&qh, 1..=1, ())
        .ok();
    let fractional = viewport.as_ref().and_then(|_| {
        fractional_manager
            .as_ref()
            .map(|manager| manager.get_fractional_scale(shell.surface(), &qh, ()))
    });
    let shm = Shm::bind(&globals, &qh)?;
    let pool = SlotPool::new(4096, &shm)?;
    let mut pin = Pin {
        connection: connection.clone(),
        registry: RegistryState::new(&globals),
        outputs,
        seats: SeatState::new(&globals, &qh),
        compositor,
        xdg,
        layers,
        shell,
        viewporter,
        viewport,
        fractional_manager,
        fractional,
        shm,
        pool,
        buffers: Vec::new(),
        image,
        output,
        bounds,
        position,
        initial_position,
        initialized: false,
        placement: Placement {
            logical: Geometry::new(position, size),
            visible: Geometry::new(position, size),
            offset: (0, 0),
            margin: position,
        },
        size,
        render_scale: info.map_or(1.0, |info| info.scale_factor.max(1) as f32),
        configured: false,
        dirty: true,
        frame_pending: false,
        exit: false,
        pointer_seat: None,
        pointer: None,
        keyboard: None,
        cursor_shapes: CursorShapeManager::bind(&globals, &qh).ok(),
        cursor: None,
        serial: 0,
        mods: Mods::default(),
        drag: None,
        native_resizing: false,
        preview: None,
        clicks: Default::default(),
        popup: None,
        font_system: crate::fonts::bundled(),
        swash: cosmic_text::SwashCache::new(),
        icons: crate::app::load_icons_cache(),
        text: Default::default(),
        text_selection: false,
        selecting: false,
        generation: 0,
        ocr_cancel: None,
        ocr_sender: sender,
        copy_after_ocr: false,
        translation: crate::features::pinned_image::translation::Translation::new(
            crate::config::get().translation.target_language.clone(),
        ),
        translation_task: None,
        translation_retired: Vec::new(),
        feedback: Default::default(),
        translation_wake,
        last_position: (0.0, 0.0),
    };
    pin.resize(false, &qh);
    if let Some(seed) = seed {
        pin.apply_translation_seed(seed);
    }
    let visibility_wake = pin.translation_wake.clone();
    let mut visibility =
        crate::backend::capture_visibility::Watch::new(move || visibility_wake.ping())?;
    let mut hidden = false;
    while !pin.exit {
        let requested = visibility.requested()?;
        if hidden != requested {
            if requested {
                pin.popup = None;
                pin.preview = None;
                pin.shell.surface().attach(None, 0, 0);
                pin.shell.surface().commit();
                pin.configured = false;
                pin.frame_pending = false;
            } else {
                pin.recreate(&qh);
                pin.resize(false, &qh);
            }
            connection.flush()?;
            hidden = requested;
        }
        visibility.applied()?;
        if hidden {
            event_loop.dispatch(Some(std::time::Duration::from_millis(100)), &mut pin)?;
            continue;
        }
        pin.dirty |= pin.feedback.expire();
        pin.poll_translation();
        pin.render_preview(&qh)?;
        if pin.dirty && pin.configured && !pin.frame_pending {
            pin.render(&qh)?;
        }
        let timeout = [
            (!pin.translation_retired.is_empty()).then_some(std::time::Duration::from_millis(10)),
            pin.feedback
                .deadline()
                .map(|deadline| deadline.saturating_duration_since(std::time::Instant::now())),
        ]
        .into_iter()
        .flatten()
        .min();
        event_loop.dispatch(timeout, &mut pin)?;
    }
    Ok(())
}
