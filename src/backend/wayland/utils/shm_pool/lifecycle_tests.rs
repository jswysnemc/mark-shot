//! 【共享内存测试】【释放生命周期】使用真实协议验证活动隔离、延迟销毁与断开连接后的引用归还

use super::{FreelistEntry, test_server::Server};
use crate::backend::wayland::overlay::state::OverlayRunTime;
use wayland_client::protocol::wl_shm;

/// 【共享内存测试】【先释放后销毁】无参数，验证重复激活被拒绝且真实释放事件恢复可写能力与槽位复用，无返回值
#[test]
fn release_restores_canvas_and_drop_reuses_the_slot() {
    let (server, connection) = Server::new();
    let mut runtime = OverlayRunTime::new(&connection).unwrap();
    let surface = runtime
        .state
        .compositor_state
        .create_surface(&runtime.event_queue.handle());
    let (buffer, pixels) = runtime
        .state
        .pool
        .create_buffer(64, 64, 256, wl_shm::Format::Argb8888)
        .unwrap();
    pixels.fill(37);
    let offset = buffer.slot.inner.offset;
    buffer.attach_to(&surface).unwrap();
    surface.commit();
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    assert!(buffer.activate().is_err());
    assert!(runtime.state.pool.canvas(&buffer).is_none());
    // 1. 【共享内存测试】【占用解除】真实释放事件解除占用，像素仍保留在原槽位
    server.release(0);
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    assert!(
        runtime
            .state
            .pool
            .canvas(&buffer)
            .unwrap()
            .iter()
            .all(|&value| value == 37)
    );
    drop(buffer);
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    assert_eq!(server.snapshot().destroyed_buffers, [0]);
    let slot = runtime.state.pool.new_slot(64 * 64 * 4).unwrap();
    assert_eq!(slot.inner.offset, offset);
    surface.destroy();
}

/// 【共享内存测试】【先销毁后释放】无参数，验证活动缓冲丢弃后延迟发送 destroy，释放前槽位不能重用，无返回值
#[test]
fn dropped_active_buffer_waits_for_release_before_reuse() {
    let (server, connection) = Server::new();
    let mut runtime = OverlayRunTime::new(&connection).unwrap();
    let surface = runtime
        .state
        .compositor_state
        .create_surface(&runtime.event_queue.handle());
    let (buffer, _) = runtime
        .state
        .pool
        .create_buffer(256, 256, 1024, wl_shm::Format::Argb8888)
        .unwrap();
    let inner = buffer.slot.inner.clone();
    buffer.attach_to(&surface).unwrap();
    surface.commit();
    drop(buffer);
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    assert!(server.snapshot().destroyed_buffers.is_empty());
    let pending = runtime.state.pool.new_slot(256 * 256 * 4).unwrap();
    assert_ne!(pending.inner.offset, inner.offset);
    // 1. 【共享内存测试】【延迟销毁】丢弃所有者并不代表服务端已释放，只有 release 允许协议销毁与区间归还
    server.release(0);
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    assert_eq!(server.snapshot().destroyed_buffers, [0]);
    assert_eq!(
        inner
            .active_buffers
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
    assert_eq!(inner.all_refs.load(std::sync::atomic::Ordering::Relaxed), 0);
    let reused = runtime.state.pool.new_slot(256 * 256 * 4).unwrap();
    assert_eq!(reused.inner.offset, inner.offset);
    surface.destroy();
}

/// 【共享内存测试】【共享槽位】无参数，验证同一槽位的两个协议缓冲全部释放后才恢复写入能力，无返回值
#[test]
fn aliased_slot_stays_busy_until_every_buffer_is_released() {
    let (server, connection) = Server::new();
    let mut runtime = OverlayRunTime::new(&connection).unwrap();
    let first_surface = runtime
        .state
        .compositor_state
        .create_surface(&runtime.event_queue.handle());
    let second_surface = runtime
        .state
        .compositor_state
        .create_surface(&runtime.event_queue.handle());
    let slot = runtime.state.pool.new_slot(64 * 64 * 4).unwrap();
    let first = runtime
        .state
        .pool
        .create_buffer_in(&slot, 64, 64, 256, wl_shm::Format::Argb8888)
        .unwrap();
    let second = runtime
        .state
        .pool
        .create_buffer_in(&slot, 64, 64, 256, wl_shm::Format::Argb8888)
        .unwrap();
    first.attach_to(&first_surface).unwrap();
    second.attach_to(&second_surface).unwrap();
    first_surface.commit();
    second_surface.commit();
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    server.release(0);
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    assert!(runtime.state.pool.canvas(&slot).is_none());
    server.release(1);
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    assert!(runtime.state.pool.canvas(&slot).is_some());
    first_surface.destroy();
    second_surface.destroy();
}

/// 【共享内存测试】【连接退出】无参数，验证未收到 release 的缓冲在连接关闭时归还活动计数及逻辑引用，无返回值
#[test]
fn connection_shutdown_reclaims_an_unreleased_dropped_buffer() {
    let (_server, connection) = Server::new();
    let mut runtime = OverlayRunTime::new(&connection).unwrap();
    let surface = runtime
        .state
        .compositor_state
        .create_surface(&runtime.event_queue.handle());
    let (buffer, _) = runtime
        .state
        .pool
        .create_buffer(64, 64, 256, wl_shm::Format::Argb8888)
        .unwrap();
    let inner = buffer.slot.inner.clone();
    let free_list = runtime.state.pool.free_list.clone();
    buffer.attach_to(&surface).unwrap();
    surface.commit();
    drop(buffer);
    runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    drop(surface);
    drop(runtime);
    drop(connection);
    assert_eq!(
        inner
            .active_buffers
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
    assert_eq!(inner.all_refs.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert_eq!(
        *free_list.lock().unwrap(),
        [FreelistEntry {
            offset: 0,
            len: 256 * 1024
        }]
    );
}

/// 【共享内存测试】【连续整帧】无参数，验证报告尺寸的连续帧在释放后复用同一偏移，扩容不会恢复密封能力，无返回值
#[test]
fn repeated_full_frames_reuse_capacity_without_shrink_seals() {
    let (server, connection) = Server::new();
    let mut runtime = OverlayRunTime::new(&connection).unwrap();
    let surface = runtime
        .state
        .compositor_state
        .create_surface(&runtime.event_queue.handle());
    for index in 0..32 {
        let (buffer, _) = runtime
            .state
            .pool
            .create_buffer(2561, 1440, 2561 * 4, wl_shm::Format::Argb8888)
            .unwrap();
        assert_eq!(buffer.slot.inner.offset, 0);
        buffer.attach_to(&surface).unwrap();
        surface.commit();
        drop(buffer);
        runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
        server.release(index);
        runtime.event_queue.roundtrip(&mut runtime.state).unwrap();
    }
    let snapshot = server.snapshot();
    assert_eq!(snapshot.pool_sizes, [2561 * 1440 * 4]);
    assert_eq!(snapshot.pool_seals, [nix::libc::F_SEAL_SEAL]);
    assert_eq!(snapshot.destroyed_buffers.len(), 32);
    assert_eq!(snapshot.buffer_offsets, vec![0; 32]);
    surface.destroy();
}
