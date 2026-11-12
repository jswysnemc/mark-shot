//! 【共享缓冲测试】【真实槽位】独立 Wayland 连接验证活动来源隔离、局部复制和回收失效

use super::*;

/// 【共享缓冲测试】【局部与回收】无参数，使用真实共享池验证范围外像素、活动来源只读及回收后完整重建
#[test]
#[ignore = "requires MARK_SHOT_TEST_WAYLAND_DISPLAY on an isolated compositor"]
fn partial_copy_preserves_active_source_and_rebuilds_reclaimed_spare() {
    let display =
        std::env::var("MARK_SHOT_TEST_WAYLAND_DISPLAY").expect("private display required");
    assert_eq!(std::env::var("WAYLAND_DISPLAY").unwrap(), display);
    let connection = wayland_client::Connection::connect_to_env().unwrap();
    let mut runtime =
        crate::backend::wayland::overlay::state::OverlayRunTime::new(&connection).unwrap();
    let pool = &mut runtime.state.pool;
    let mut source = create_shm_buffer(pool, 1024, 700).unwrap();
    let mut target = create_shm_buffer(pool, 1024, 700).unwrap();
    let expected: Vec<u8> = (0..1024 * 700 * 4)
        .map(|index| ((index * 37 + index / 1024) % 251) as u8)
        .collect();
    source
        .write(pool, |pixels| {
            pixels.copy_from_slice(&expected);
            Ok(())
        })
        .unwrap();
    target
        .write(pool, |pixels| {
            pixels.fill(93);
            Ok(())
        })
        .unwrap();
    let surface = runtime
        .state
        .compositor_state
        .create_surface(&runtime.event_queue.handle());
    source.buffer.attach_to(&surface).unwrap();
    assert!(source.active());
    assert!(source.write(pool, |_| Ok(())).is_err());
    // 1. 【共享缓冲测试】【真实差异】来源仍活动时只复制窄区域，目的范围之外保留原值
    target
        .copy_region(pool, &source, Some((31, 17, 79, 53)))
        .unwrap();
    let actual = pool.canvas(&target.buffer).unwrap();
    for y in 0..700 {
        for x in 0..1024 {
            let first = (y * 1024 + x) * 4;
            if (31..110).contains(&x) && (17..70).contains(&y) {
                assert_eq!(&actual[first..first + 4], &expected[first..first + 4]);
            } else {
                assert_eq!(&actual[first..first + 4], &[93; 4]);
            }
        }
    }
    // 2. 【共享缓冲测试】【内容失效】已回收备用槽位不能沿用局部差异，必须重新复制完整活动来源
    target.reclaim(pool);
    assert!(target.reclaimed());
    target
        .copy_region(pool, &source, Some((31, 17, 79, 53)))
        .unwrap();
    assert_eq!(pool.canvas(&target.buffer).unwrap(), expected);
    assert_eq!(pool.raw_data_mut(&source.buffer.slot()), expected);
    assert!(!target.reclaimed());
    surface.destroy();
}
