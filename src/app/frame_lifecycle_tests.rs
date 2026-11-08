//! 【帧生命周期测试】【连续渲染】消费源前景后，实际完整帧、局部更新和等待期间累积损坏仍与保留源路径相同

use super::{
    frame_lifecycle_overlay::Overlay,
    frame_render::{Pacing, clear_frame_state, render_dirty_monitors},
    region_rendering_test_support as support,
};
use crate::{
    editor::{DamageZone, EditorState},
    tools::Tool,
    types::{AnnotationShape, MouseButton},
};
use tiny_skia::Rect;

struct Harness {
    state: EditorState,
    overlay: Overlay,
    pacing: Pacing,
}

impl Harness {
    /// 【帧生命周期测试】【环境构造】按scale和consumed创建相同纹理的编辑器及实际像素后端，返回测试环境
    fn new(scale: f32, consumed: bool) -> Self {
        let mut state = support::editor(scale, true);
        state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
        let mut overlay = Overlay::new(&state.canvas, consumed);
        super::frame_backdrop::configure(&mut state, &mut overlay, true);
        Self {
            state,
            overlay,
            pacing: Pacing::new(1),
        }
    }

    /// 【帧生命周期测试】【真实帧入口】按dirty和hidden执行编辑器实际重建、节奏、提交及帧末清理，无返回值
    fn render(&mut self, mut dirty: u32, hidden: bool) {
        render_dirty_monitors(
            &mut self.state,
            &mut self.overlay,
            dirty,
            hidden,
            &mut self.pacing,
        )
        .unwrap();
        clear_frame_state(&mut self.state, &mut dirty, &self.pacing);
    }

    /// 【帧生命周期测试】【矩形更新】按start和end插入或移动半透明正文，登记旧、新损坏区域，无返回值
    fn rectangle(&mut self, start: (f32, f32), end: (f32, f32)) {
        if let Some(previous) = self.state.annotations.first() {
            self.state
                .damage_rects
                .push(DamageZone::Global(previous.damage_bbox(false)));
        }
        let annotation = support::annotation(
            1,
            0.0,
            AnnotationShape::Rectangle {
                radius: 0.0,
                fixed_ratio: false,
                start,
                end,
                filled: true,
            },
        );
        self.state
            .damage_rects
            .push(DamageZone::Global(annotation.damage_bbox(false)));
        self.state.annotations.clear();
        self.state.annotations.push(annotation);
        self.state.annotations_dirty = true;
    }
}

/// 【帧生命周期测试】【原生字节对照】按actual和expected比较每个输出的完整BGRA，无返回值，失败保留首个字节位置
fn equal(actual: &Harness, expected: &Harness) {
    for (monitor, (first, second)) in actual
        .overlay
        .frames
        .iter()
        .zip(&expected.overlay.frames)
        .enumerate()
    {
        let difference = first.iter().zip(second).position(|(a, b)| a != b);
        assert_eq!(difference, None, "native output {monitor} differs");
    }
}

/// 【帧生命周期测试】【首次局部通知】无参数，首次仅收到面板损坏时仍恢复完整选区外背景，消费与只读及分数比例均一致，无返回值
#[test]
fn first_frame_restores_the_whole_scene_when_only_a_panel_is_marked_dirty() {
    for scale in [1.0, 1.2, 1.5] {
        for consumed in [false, true] {
            let mut actual = Harness::new(scale, consumed);
            let mut expected = Harness::new(scale, consumed);
            for harness in [&mut actual, &mut expected] {
                harness.state.selection.zone = Rect::from_xywh(0.0, 0.0, 633.0, 480.0);
                harness.state.selection.prev_zone = harness.state.selection.zone;
                crate::utils::copy_swizzled(
                    &mut harness.overlay.frames[0],
                    harness.state.captures[0].pixmap.data(),
                );
            }
            actual.state.damage_rects.push(DamageZone::Local {
                monitor_idx: 0,
                rect: Rect::from_xywh(0.0, 0.0, 200.0, 60.0).unwrap(),
            });
            actual.render(1, false);
            expected.render(1, false);
            equal(&actual, &expected);
        }
    }
}

/// 【帧生命周期测试】【首次延迟范围】无参数，首次等待原生可写时累积多个局部通知，提交后保留完整背景，后续局部更新继续一致，无返回值
#[test]
fn first_frame_keeps_whole_scene_damage_while_waiting_for_a_native_buffer() {
    let mut actual = Harness::new(1.5, true);
    let mut expected = Harness::new(1.5, false);
    for harness in [&mut actual, &mut expected] {
        harness.state.selection.zone = Rect::from_xywh(0.0, 0.0, 633.0, 480.0);
        harness.state.selection.prev_zone = harness.state.selection.zone;
        crate::utils::copy_swizzled(
            &mut harness.overlay.frames[0],
            harness.state.captures[0].pixmap.data(),
        );
    }
    actual.overlay.ready = false;
    for point in [(30.0, 40.0), (220.0, 340.0)] {
        actual.state.damage_rects.push(DamageZone::Local {
            monitor_idx: 0,
            rect: Rect::from_xywh(point.0, point.1, 20.0, 20.0).unwrap(),
        });
        actual.render(1, false);
    }
    assert_eq!(actual.overlay.sent, 0);
    actual.overlay.ready = true;
    actual.render(0, false);
    expected.render(1, false);
    equal(&actual, &expected);
    for harness in [&mut actual, &mut expected] {
        harness.rectangle((80.0, 100.0), (250.0, 240.0));
        harness.render(1, false);
    }
    equal(&actual, &expected);
}

/// 【帧生命周期测试】【连续局部重建】无参数，多种比例下完整提交后消费CPU源，局部正文和选区更新仍保持全部原生像素，无返回值
#[test]
fn consumed_foreground_survives_repeated_annotation_and_selection_damage() {
    for scale in [1.0, 1.2, 1.5] {
        let mut actual = Harness::new(scale, true);
        let mut expected = Harness::new(scale, false);
        for harness in [&mut actual, &mut expected] {
            harness.render(1, false);
        }
        assert!(actual.state.canvas[0].data().iter().all(|byte| *byte == 0));
        equal(&actual, &expected);
        for (start, end) in [
            ((70.0, 80.0), (170.0, 130.0)),
            ((130.0, 120.0), (300.0, 210.0)),
            ((50.0, 90.0), (190.0, 180.0)),
        ] {
            for harness in [&mut actual, &mut expected] {
                harness.rectangle(start, end);
                harness.render(1, false);
            }
            equal(&actual, &expected);
        }
        for zone in [
            Rect::from_xywh(30.0, 40.0, 530.0, 380.0),
            Rect::from_xywh(50.0, 70.0, 490.0, 320.0),
            None,
        ] {
            for harness in [&mut actual, &mut expected] {
                harness.state.selection.zone = zone;
                harness.render(1, false);
            }
            equal(&actual, &expected);
        }
    }
}

/// 【帧生命周期测试】【延迟提交】无参数，等待原生可写期间多个移动先重建并累积损坏，允许提交后必须恢复完整终态，无返回值
#[test]
fn consumed_foreground_preserves_damage_owed_while_native_buffers_are_active() {
    let mut actual = Harness::new(1.5, true);
    let mut expected = Harness::new(1.5, false);
    for harness in [&mut actual, &mut expected] {
        harness.render(1, false);
    }
    let previous = actual.overlay.sent;
    actual.overlay.ready = false;
    for (start, end) in [
        ((30.0, 60.0), (150.0, 190.0)),
        ((340.0, 150.0), (450.0, 350.0)),
    ] {
        for harness in [&mut actual, &mut expected] {
            harness.rectangle(start, end);
            harness.render(1, false);
        }
    }
    assert_eq!(actual.overlay.sent, previous);
    assert!(actual.pacing.owes_to(0));
    actual.overlay.ready = true;
    actual.render(0, false);
    equal(&actual, &expected);
}

/// 【帧生命周期测试】【真实画笔】无参数，连续真实画笔的增量烘焙、缓存恢复和完整提交均不依赖已消费CPU帧，无返回值
#[test]
fn consumed_foreground_keeps_pen_baking_restore_and_commit_pixels() {
    for tool in [Tool::Pen, Tool::Highlighter] {
        let mut actual = Harness::new(1.0, true);
        let mut expected = Harness::new(1.0, false);
        for harness in [&mut actual, &mut expected] {
            let mut dirty = 0;
            super::input::select_tool(&mut harness.state, tool, &mut dirty);
            support::pointer(
                &mut harness.state,
                (180.0, 190.0),
                Some((MouseButton::Left, true)),
                &mut dirty,
            );
            harness.render(dirty, false);
        }
        equal(&actual, &expected);
        for position in [
            (200.0, 200.0),
            (260.0, 210.0),
            (230.0, 240.0),
            (320.0, 260.0),
            (400.0, 210.0),
        ] {
            for harness in [&mut actual, &mut expected] {
                let mut dirty = 0;
                support::pointer(&mut harness.state, position, None, &mut dirty);
                harness.render(dirty, false);
            }
            equal(&actual, &expected);
        }
        for harness in [&mut actual, &mut expected] {
            let mut dirty = 0;
            support::pointer(
                &mut harness.state,
                (400.0, 210.0),
                Some((MouseButton::Left, false)),
                &mut dirty,
            );
            harness.render(dirty, false);
        }
        equal(&actual, &expected);
    }
}

/// 【帧生命周期测试】【导航与圆角】无参数，文件圆角、缩放导航及正文显隐切换均保持完整帧，不依赖先前CPU像素，无返回值
#[test]
fn consumed_foreground_keeps_file_navigation_and_hidden_annotation_pixels() {
    let mut actual = Harness::new(1.5, true);
    let mut expected = Harness::new(1.5, false);
    for harness in [&mut actual, &mut expected] {
        let bounds = Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap();
        let original = harness.state.captures[0].pixmap.clone().into_pixmap();
        harness.state.file_canvas = crate::editor::file_canvas::FileCanvas::new(original, bounds);
        harness.state.selection.zone = Rect::from_xywh(40.0, 50.0, 560.0, 380.0);
        harness.rectangle((80.0, 90.0), (330.0, 210.0));
        harness.render(1, false);
    }
    equal(&actual, &expected);
    for hidden in [true, false] {
        for harness in [&mut actual, &mut expected] {
            harness.render(1, hidden);
        }
        equal(&actual, &expected);
    }
    for factor in [1.5, 0.8, 1.25] {
        for harness in [&mut actual, &mut expected] {
            assert!(harness.state.ensure_navigation());
            let available = harness.state.navigation_policy().available();
            harness.state.navigation.view.as_mut().unwrap().zoom_at(
                factor,
                (300.0, 200.0),
                available,
            );
            let mut dirty = 0;
            super::canvas_navigation::damage::changed(&mut harness.state, &mut dirty);
            harness.render(dirty, false);
        }
        equal(&actual, &expected);
    }
    for harness in [&mut actual, &mut expected] {
        harness.state.navigation.view.as_mut().unwrap().reset(true);
        let mut dirty = 0;
        super::canvas_navigation::damage::changed(&mut harness.state, &mut dirty);
        harness.render(dirty, false);
        harness
            .state
            .damage_local(0, Rect::from_xywh(30.0, 40.0, 120.0, 90.0).unwrap());
        harness.render(1, false);
    }
    equal(&actual, &expected);
}

/// 【帧生命周期测试】【混合输出】无参数，独立比例及全局坐标输出的局部更新保持未提交输出的完整原生帧，无返回值
#[test]
fn consumed_foreground_keeps_independent_mixed_output_frames() {
    let mut actual = Harness::new(1.0, true);
    let mut expected = Harness::new(1.0, false);
    for harness in [&mut actual, &mut expected] {
        let mut second = support::editor_size((360, 240), 1.5, true);
        second.placements[0].position = (640, 0);
        harness.state.placements.push(second.placements.remove(0));
        harness.state.captures.push(second.captures.remove(0));
        harness.state.canvas.push(second.canvas.remove(0));
        harness.state.dimmed.push(second.dimmed.remove(0));
        harness
            .state
            .annotations_layer
            .push(second.annotations_layer.remove(0));
        harness.state.selection.zone = Rect::from_xywh(0.0, 0.0, 1000.0, 480.0);
        harness.overlay = Overlay::new(&harness.state.canvas, harness.overlay.consumed);
        harness.pacing = Pacing::new(2);
        harness.render(3, false);
    }
    equal(&actual, &expected);
    let original_first = actual.overlay.frames[0].clone();
    for (start, end) in [
        ((680.0, 60.0), (780.0, 170.0)),
        ((820.0, 80.0), (970.0, 200.0)),
    ] {
        for harness in [&mut actual, &mut expected] {
            harness.rectangle(start, end);
            harness.render(2, false);
        }
        equal(&actual, &expected);
        assert_eq!(actual.overlay.frames[0], original_first);
    }
}
