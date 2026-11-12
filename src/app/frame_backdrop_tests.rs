//! 【原生背景测试】【前景保存】实际编辑器渲染后仍保留原始透明前景，不把冻结原图写回CPU画布

use crate::app::region_rendering_test_support::{editor, paint};
use tiny_skia::Rect;

/// 【原生背景测试】【提交前景】无参数，原生背景模式下提交前的CPU前景必须与编辑器帧完全相同，无返回值
#[test]
fn native_background_keeps_the_rendered_cpu_foreground() {
    let mut editor = editor(1.0, true);
    editor.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    paint(&mut editor, None, false);
    let foreground = editor.canvas[0].clone();
    assert_eq!(foreground.pixel(300, 240).unwrap().alpha(), 0);
    editor.region_rendering.opaque_background = true;
    editor.region_rendering.native_background = true;
    super::compose(&mut editor, 0, None);
    crate::app::region_rendering_test_support::equal(
        &editor.canvas[0],
        &foreground,
        "native foreground",
    );
}

/// 【原生背景测试】【检查许可】实际场景交付在禁止回收时保留零页，允许回收后归还全零页，两次均保留可见像素
#[cfg(target_os = "linux")]
#[test]
fn native_scene_respects_reclamation_permission_after_successful_submission() {
    use std::os::unix::fs::FileExt;
    // 1. 【原生背景测试】【独占像素】写入整幅零值建立实际自有页，并保留一个不透明前景像素
    let mut state = editor(1.0, true);
    state.region_rendering.native_scene = true;
    state.canvas[0].data_mut().fill(0);
    state.canvas[0].data_mut()[..4].copy_from_slice(&[91, 110, 20, 255]);
    let expected = state.canvas[0].data().to_vec();
    let page = unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) } as usize;
    let address = state.canvas[0].data().as_ptr() as usize;
    let position = address.div_ceil(page) + 1;
    let entries = std::fs::File::open("/proc/self/pagemap").unwrap();
    let private = || {
        let mut value = [0; 8];
        entries
            .read_exact_at(&mut value, (position * 8) as u64)
            .unwrap();
        let value = u64::from_ne_bytes(value);
        value & ((1 << 63) | (1 << 56)) == ((1 << 63) | (1 << 56))
    };
    assert!(private());
    let mut overlay = crate::app::frame_lifecycle_overlay::Overlay::new(&state.canvas, false);
    // 2. 【原生背景测试】【交付隔离】禁止检查时像素已经进入实际原生场景，但CPU零页仍保持独占
    super::stage(&mut state, &mut overlay, 0, None, false).unwrap();
    assert!(private(), "zero pages were reclaimed without permission");
    assert!(
        state.canvas[0].data() == expected,
        "foreground bytes changed"
    );
    let native = overlay.frames[0].clone();
    // 3. 【原生背景测试】【透明归还】允许检查后只归还全零页，原生帧与非零前景保持完全相同
    super::stage(&mut state, &mut overlay, 0, None, true).unwrap();
    assert!(!private());
    assert!(
        state.canvas[0].data() == expected,
        "foreground bytes changed"
    );
    assert!(overlay.frames[0] == native, "submitted scene changed");
}
