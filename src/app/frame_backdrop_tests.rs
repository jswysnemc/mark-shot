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

/// 【原生背景测试】【非整数缩放移动】验证 125%/150%/175% 下区域框选与高亮范围严格重合，移动选区后不残留旧高亮孔洞
#[cfg(target_os = "linux")]
#[test]
fn fractional_scale_selection_move_aligns_highlight_and_clears_previous_hole() {
    use crate::{
        app::{
            frame_lifecycle_overlay::Overlay,
            frame_render::{Pacing, clear_frame_state, render_dirty_monitors},
            region_rendering_test_support as support,
        },
        tools::Tool,
        types::MouseButton,
    };

    for scale in [1.25, 1.5, 1.75] {
        let mut incremental = support::editor(scale, true);
        let mut full_scene = support::editor(scale, true);
        let mut legacy = support::editor(scale, true);
        incremental.region_rendering.native_scene = true;
        full_scene.region_rendering.native_scene = true;
        for state in [&mut incremental, &mut full_scene, &mut legacy] {
            state.ui_hidden = true;
            state.selected_tool = Tool::Selection;
            state.selection.zone = Rect::from_xywh(90.0, 70.0, 220.0, 160.0);
        }
        let mut inc_overlay = Overlay::new(&incremental.canvas, false);
        let mut full_overlay = Overlay::new(&full_scene.canvas, false);
        super::configure(&mut incremental, &mut inc_overlay, true);
        super::configure(&mut full_scene, &mut full_overlay, true);
        let mut inc_pacing = Pacing::new(1);
        let mut full_pacing = Pacing::new(1);

        let mut dirty = 1;
        render_dirty_monitors(
            &mut incremental,
            &mut inc_overlay,
            dirty,
            true,
            &mut inc_pacing,
        )
        .unwrap();
        clear_frame_state(&mut incremental, &mut dirty, &inc_pacing);

        // 1. 【原生背景测试】【拖动选区】连续移动选框验证局部损坏更新，旧高亮区域必须随移动完整恢复遮罩
        for state in [&mut incremental, &mut full_scene, &mut legacy] {
            support::pointer(
                state,
                (150.0, 120.0),
                Some((MouseButton::Left, true)),
                &mut 0,
            );
        }
        let mut press_dirty = 1;
        render_dirty_monitors(
            &mut incremental,
            &mut inc_overlay,
            press_dirty,
            true,
            &mut inc_pacing,
        )
        .unwrap();
        clear_frame_state(&mut incremental, &mut press_dirty, &inc_pacing);
        incremental.damage_rects.clear();
        full_scene.damage_rects.clear();

        for target in [(210.0, 175.0), (290.0, 230.0), (120.0, 95.0)] {
            let mut move_dirty = 0;
            support::pointer(&mut incremental, target, None, &mut move_dirty);
            support::pointer(&mut full_scene, target, None, &mut 0);
            support::pointer(&mut legacy, target, None, &mut 0);
            render_dirty_monitors(
                &mut incremental,
                &mut inc_overlay,
                move_dirty,
                true,
                &mut inc_pacing,
            )
            .unwrap();
            clear_frame_state(&mut incremental, &mut move_dirty, &inc_pacing);

            let full_rect = full_scene.placements[0].rect().unwrap();
            full_scene.damage_local(0, full_rect);
            let mut full_dirty = 1;
            render_dirty_monitors(
                &mut full_scene,
                &mut full_overlay,
                full_dirty,
                true,
                &mut full_pacing,
            )
            .unwrap();
            clear_frame_state(&mut full_scene, &mut full_dirty, &full_pacing);
            assert_eq!(
                inc_overlay.frames[0], full_overlay.frames[0],
                "stale highlight remained at scale={scale}, target={target:?}"
            );

            // 2. 【原生背景测试】【传统遮罩对照】对比传统 dimmed 层全屏合成结果，确保物理像素高亮范围一致
            support::paint(&mut legacy, None, true);
            let mut legacy_rgba = legacy.canvas[0].clone();
            crate::renderer::opaque_backdrop::compose(
                &legacy.captures[0].pixmap,
                &mut legacy_rgba,
                None,
            );
            let max_diff = inc_overlay.frames[0]
                .as_chunks::<4>()
                .0
                .iter()
                .zip(legacy_rgba.data().as_chunks::<4>().0)
                .map(|(bgra, rgba)| {
                    [
                        bgra[2].abs_diff(rgba[0]),
                        bgra[1].abs_diff(rgba[1]),
                        bgra[0].abs_diff(rgba[2]),
                        bgra[3].abs_diff(rgba[3]),
                    ]
                    .into_iter()
                    .max()
                    .unwrap()
                })
                .max()
                .unwrap();
            assert!(
                max_diff <= 2,
                "highlight misaligned with selection border at scale={scale}: max_diff={max_diff}"
            );
        }
    }
}
