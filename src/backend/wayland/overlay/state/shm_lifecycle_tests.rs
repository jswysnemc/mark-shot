//! 【截图覆盖层测试】【配置生命周期】通过真实配置事件检查探测表面与滚动父窗口的占用登记

use super::{OverlayRunTime, Probe};
use crate::backend::wayland::utils::{
    scroll_popup::ScrollPopup,
    shm_pool::test_server::Server,
    surface::{InputRegion, Role, SurfaceData},
};
use smithay_client_toolkit::shell::{WaylandSurface, wlr_layer::Layer};

/// 【截图覆盖层测试】【探测占用】无参数，验证探测像素提交后保持活动，所有者丢弃时等待真实释放，无返回值
#[test]
fn monitor_probe_registers_its_buffer_until_release() {
    let (server, connection) = Server::new();
    let mut runtime = OverlayRunTime::new(&connection).unwrap();
    let qh = runtime.event_queue.handle();
    let layer = runtime
        .state
        .layer_shell
        .as_ref()
        .unwrap()
        .create_layer_surface(
            &qh,
            runtime.state.compositor_state.create_surface(&qh),
            Layer::Overlay,
            Some("mark-shot-test"),
            None,
        );
    layer.commit();
    runtime.state.probe = Some(Probe {
        layer,
        buffer: None,
        output: None,
    });
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    server.configure_layer();
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    // 1. 【截图覆盖层测试】【活动登记】配置回调实际提交缓冲，客户端必须同步登记服务端占用
    let buffer = runtime
        .state
        .probe
        .as_ref()
        .unwrap()
        .buffer
        .as_ref()
        .unwrap();
    assert!(buffer.active(), "probe attachment bypassed activation");
    assert_eq!(server.snapshot().attached_buffers, [0]);
    runtime.state.probe.as_mut().unwrap().buffer = None;
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    assert!(server.snapshot().destroyed_buffers.is_empty());
    server.release(0);
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    assert_eq!(server.snapshot().destroyed_buffers, [0]);
}

/// 【截图覆盖层测试】【父窗口状态】按 runtime 建立透明滚动父窗口并注册配置状态，无返回值
fn create_scroll_parent(runtime: &mut OverlayRunTime) {
    let qh = runtime.event_queue.handle();
    let parent = ScrollPopup::create_parent(
        &runtime.state.compositor_state,
        &runtime.state.xdg_shell,
        &qh,
    );
    let surface = parent.wl_surface().clone();
    surface.commit();
    runtime.state.surfaces.insert(
        0,
        SurfaceData {
            background: None,
            scroll_layer: None,
            role: Role::ScrollParent(parent),
            surface,
            viewport: None,
            shm_buffer: None,
            spare_buffer: None,
            buffer_history: Default::default(),
            waiting_release: false,
            mapped: false,
            input: InputRegion::Nothing,
            canvas_rect: None,
            capture_logical_size: None,
            capture_buffer_size: None,
            fullscreen_output: None,
            bootstrap_size: None,
            bootstrap_buffer: None,
            scroll_frame: None,
            scroll_stamps: Default::default(),
            width: 1,
            height: 1,
            scale: 1.0,
            frame_sent: None,
        },
    );
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
}

/// 【截图覆盖层测试】【重复配置】无参数，验证初次配置只附着一次，重复尺寸不重新附着仍活动的透明父缓冲，无返回值
#[test]
fn scroll_parent_configure_does_not_reattach_an_active_buffer() {
    let (server, connection) = Server::new();
    let mut runtime = OverlayRunTime::new(&connection).unwrap();
    create_scroll_parent(&mut runtime);
    server.configure_window(1, 1);
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    assert_eq!(
        server.snapshot().attached_buffers,
        [0],
        "first configure attached the same buffer twice"
    );
    assert!(
        runtime.state.surfaces[&0]
            .shm_buffer
            .as_ref()
            .unwrap()
            .active()
    );
    server.configure_window(1, 1);
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    assert_eq!(server.snapshot().attached_buffers, [0]);
    assert!(runtime.state.configure_error.is_none());
    // 1. 【截图覆盖层测试】【几何复用】服务端已释放当前缓冲时，同尺寸配置同样只提交几何状态，不需要重新附着像素
    server.release(0);
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    server.configure_window(1, 1);
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    assert_eq!(server.snapshot().attached_buffers, [0]);
}

/// 【截图覆盖层测试】【父窗口扩容】无参数，验证尺寸变化使用独立活动缓冲，旧槽位等待 release 才销毁，无返回值
#[test]
fn scroll_parent_resize_preserves_the_unreleased_old_slot() {
    let (server, connection) = Server::new();
    let mut runtime = OverlayRunTime::new(&connection).unwrap();
    create_scroll_parent(&mut runtime);
    server.configure_window(1, 1);
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    server.configure_window(128, 128);
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    let snapshot = server.snapshot();
    assert_eq!(snapshot.attached_buffers, [0, 1]);
    assert_ne!(snapshot.buffer_offsets[0], snapshot.buffer_offsets[1]);
    assert!(snapshot.destroyed_buffers.is_empty());
    assert!(
        runtime.state.surfaces[&0]
            .shm_buffer
            .as_ref()
            .unwrap()
            .active()
    );
    server.release(0);
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    assert_eq!(server.snapshot().destroyed_buffers, [0]);
}
