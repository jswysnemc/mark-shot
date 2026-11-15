//! 【标注多选绘制测试】【局部像素】共同手势动态层与提交缓存逐像素一致，分数比例局部重绘不能累加控制框

use super::*;
use crate::{
    app::frame_render::{Frame, rebuild_layer, render_monitor},
    types::{AnnotationShape, SelectionHandle},
};
use tiny_skia::Rect;

/// 【标注多选绘制测试】【绘制输入】按state创建当前帧参数，成员色使用实际首项，图层与控件均可见
fn frame(state: &EditorState) -> Frame {
    Frame {
        active_text_id: None,
        scan_badge: None,
        current_color: state.annotations[0].color,
        outline_color: Color::TRANSPARENT,
        annotations_hidden: false,
    }
}

/// 【标注多选绘制测试】【分数画布】按scale返回带重叠透明对象的群组，持久缓存与画布保持真实输出像素比例
fn prepared(scale: f32) -> EditorState {
    let mut state = super::annotation_collection::group();
    state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    state.selection.prev_zone = state.selection.zone;
    state.toolbar.opacity = 0.0;
    state.action_toolbar.opacity = 0.0;
    state.settings_panel.visible = false;
    state.placements[0].scale = scale;
    let make = || {
        let mut pixmap = Pixmap::new((640.0 * scale) as u32, (480.0 * scale) as u32).unwrap();
        pixmap.fill(Color::from_rgba8(30, 34, 38, 255));
        pixmap
    };
    state.canvas[0] = make();
    state.dimmed[0] = make();
    state.captures[0].pixmap = make().into();
    state.captures[0].scale = scale;
    state.annotations_layer[0] =
        Pixmap::new((640.0 * scale) as u32, (480.0 * scale) as u32).unwrap();
    let mut overlap = state.annotations[0].clone();
    overlap.id = 91;
    overlap.shape = AnnotationShape::Rectangle {
        radius: 0.0,
        fixed_ratio: false,
        start: (220.0, 190.0),
        end: (380.0, 280.0),
        filled: true,
    };
    overlap.color = Color::from_rgba8(80, 140, 160, 120);
    overlap.update_bbox();
    state.annotations.insert(1, overlap);
    state.annotations[0].rotation_degrees = 13.0;
    state.annotations[2].rotation_degrees = -21.0;
    state.annotations[0].color.set_alpha(0.65);
    state.annotations[2].color.set_alpha(0.7);
    state.annotations_dirty = true;
    rebuild_layer(
        &mut state,
        0,
        &Frame {
            active_text_id: None,
            scan_badge: None,
            current_color: Color::BLACK,
            outline_color: Color::TRANSPARENT,
            annotations_hidden: false,
        },
    );
    state
}

/// 【标注多选绘制测试】【分段合成】按scale的当前群组移动后反复绘制损坏条带，与整帧和提交缓存完全一致
#[test]
fn fractional_group_redraws_match_the_whole_frame_and_committed_layer() {
    for scale in [1.0, 1.5, 2.0] {
        let mut state = prepared(scale);
        state.input.pointer.global = (300.0, 220.0);
        state.begin_collection_drag(SelectionHandle::Move);
        state.update_collection_drag((313.375, 231.625));
        let request = frame(&state);
        let mut dimmed_for = state.selection.zone;
        render_monitor(&mut state, 0, &request, None, &mut dimmed_for);
        let whole = state.canvas[0].clone();
        // 1. 【标注多选绘制测试】【稳定相位】多次恢复相同条带，区域外像素和白色控制点底线必须保持不变
        for x in [155.375, 179.625, 209.25, 293.375, 347.625, 426.5] {
            let dirty = Rect::from_xywh(x, 74.25, 13.625, 320.375).unwrap();
            for _ in 0..3 {
                render_monitor(&mut state, 0, &request, Some(dirty), &mut dimmed_for);
            }
            if state.canvas[0].data() != whole.data() {
                let identity = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos();
                let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                    "target/multi-selection-r1/render-failure-{identity}"
                ));
                std::fs::create_dir_all(&root).unwrap();
                whole.save_png(root.join("whole.png")).unwrap();
                state.canvas[0].save_png(root.join("sliced.png")).unwrap();
                let differences: Vec<_> = state.canvas[0]
                    .data()
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(whole.data().as_chunks::<4>().0.iter())
                    .enumerate()
                    .filter(|(_, (actual, expected))| actual != expected)
                    .map(|(index, (actual, expected))| {
                        (
                            index % whole.width() as usize,
                            index / whole.width() as usize,
                            actual.to_vec(),
                            expected.to_vec(),
                        )
                    })
                    .collect();
                panic!(
                    "scale={scale} x={x} count={} differences={:?}",
                    differences.len(),
                    &differences[..differences.len().min(20)]
                );
            }
        }
        // 2. 【标注多选绘制测试】【图层顺序】共同手势结束后，缓存合成保留未选中透明对象的原始层次
        state.finish_collection_drag();
        rebuild_layer(&mut state, 0, &request);
        render_monitor(&mut state, 0, &request, None, &mut dimmed_for);
        assert!(
            state.canvas[0].data() == whole.data(),
            "scale={scale} committed layer differs"
        );
    }
}

/// 【标注多选绘制测试】【端点状态】群组旋转后单项端点选中与历史恢复使用同一Qt圆点样式，状态变化不能改变像素
#[test]
fn reselected_line_endpoint_has_the_same_pixels_after_history_clears_its_point_state() {
    for scale in [1.0, 1.5, 2.0] {
        let mut state = prepared(scale);
        state.annotations[0].shape = AnnotationShape::Line {
            line: crate::features::line::LineSkeleton::new((180.0, 130.0), (280.0, 210.0)).unwrap(),
            style: Default::default(),
        };
        state.annotations[0].rotation_degrees = 79.0;
        state.annotations[0].update_bbox();
        state.select_annotation_ids([7]);
        let request = frame(&state);
        let mut dimmed_for = state.selection.zone;
        rebuild_layer(&mut state, 0, &request);
        render_monitor(&mut state, 0, &request, None, &mut dimmed_for);
        let neutral = state.canvas[0].clone();
        for index in 0..2 {
            state.line_session.selected_point = Some((7, index));
            render_monitor(&mut state, 0, &request, None, &mut dimmed_for);
            assert!(
                state.canvas[0].data() == neutral.data(),
                "scale={scale} point={index}"
            );
        }
    }
}
