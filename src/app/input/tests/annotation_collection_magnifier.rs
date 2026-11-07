//! 【标注多选绘制测试】【放大镜轴心】对照Qt真实边缘模型，验证正文、历史、局部重绘和提交缓存

use super::*;
use crate::{
    app::frame_render::{Frame, rebuild_layer, render_monitor},
    editor::annotation_collection::properties,
    types::{AnnotationShape, SelectionHandle},
};
use serde_json::Value;
use tiny_skia::Rect;

/// 【标注多选绘制测试】【帧参数】按state返回显示正文与共同选框的生产绘制请求
fn frame(state: &EditorState) -> Frame {
    Frame {
        active_text_id: None,
        scan_badge: None,
        current_color: state.annotations[0].color,
        outline_color: Color::TRANSPARENT,
        annotations_hidden: false,
    }
}

/// 【标注多选绘制测试】【原生输入】按members与scale构造Qt边缘成员，返回带空间变化原图和透明遮挡对象的真实编辑器
fn prepared(members: &Value, scale: f32) -> EditorState {
    let mut state =
        super::annotation_collection_reference::prepared(&serde_json::json!({"before": members}));
    state
        .annotations
        .retain(|annotation| [7, 33].contains(&annotation.id));
    state.select_annotation_ids([33, 7]);
    state.toolbar.opacity = 0.0;
    state.action_toolbar.opacity = 0.0;
    state.settings_panel.visible = false;
    state.placements[0].scale = scale;
    let mut image = Pixmap::new((640.0 * scale) as u32, (480.0 * scale) as u32).unwrap();
    let width = image.width() as usize;
    for (index, pixel) in image
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        let x = index % width;
        let y = index / width;
        pixel.copy_from_slice(&[(x % 160) as u8, (y % 120) as u8, 100, 255]);
    }
    state.canvas[0] = image.clone();
    state.dimmed[0] = image.clone();
    state.captures[0].pixmap = image.into();
    state.captures[0].scale = scale;
    state.annotations_layer[0] =
        Pixmap::new((640.0 * scale) as u32, (480.0 * scale) as u32).unwrap();
    let mut overlap = state.annotations[0].clone();
    overlap.id = 91;
    overlap.shape = AnnotationShape::Rectangle {
        start: (510.0, 35.0),
        end: (615.0, 105.0),
        filled: true,
    };
    overlap.rotation_degrees = 0.0;
    overlap.stroke_width = 3.0;
    overlap.color = Color::from_rgba8(80, 140, 160, 120);
    overlap.update_bbox();
    state.annotations.insert(1, overlap);
    state.annotations_dirty = true;
    state
}

/// 【标注多选绘制测试】【整帧绘制】以state当前模型重建缓存并绘制，返回完整像素图
fn whole(state: &mut EditorState) -> Pixmap {
    let request = frame(state);
    rebuild_layer(state, 0, &request);
    let mut dimmed_for = state.selection.zone;
    render_monitor(state, 0, &request, None, &mut dimmed_for);
    state.canvas[0].clone()
}

/// 【标注多选绘制测试】【像素比较】按context比较actual和expected，只报告首个差异像素，避免输出完整图像
fn same(actual: &Pixmap, expected: &Pixmap, context: &str) {
    assert_eq!(
        actual
            .data()
            .iter()
            .zip(expected.data())
            .position(|(a, b)| a != b),
        None,
        "{context}"
    );
}

/// 【标注多选绘制测试】【宽度与历史】边缘矩形和圆形放大镜改宽后的真实正文必须与Qt轴心模型相同，撤销重做逐像素恢复
#[test]
fn edge_magnifier_width_and_history_match_the_qt_body_model() {
    let values = super::annotation_collection_reference::reference();
    for sample in values["width"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|sample| sample["scene"].as_u64().unwrap() >= 4 && sample["requested"] == 24)
    {
        for scale in [1.0, 1.5, 2.0] {
            let context = format!("scene={} scale={scale}", sample["scene"]);
            let mut state = prepared(&sample["before"], scale);
            let before = whole(&mut state);
            assert!(properties::set_width(&mut state, 24, true, &mut 0));
            let actual = whole(&mut state);
            let mut expected = prepared(&sample["after"], scale);
            same(&actual, &whole(&mut expected), &context);
            key(&mut state, "Ctrl+Z", &mut 0);
            same(&whole(&mut state), &before, &format!("{context} undo"));
            key(&mut state, "Ctrl+Y", &mut 0);
            same(&whole(&mut state), &actual, &format!("{context} redo"));
        }
    }
}

/// 【标注多选绘制测试】【派生范围】按state检查放大镜正文bbox等于实际模型边界，旧轴心计算结果不能进入损坏与裁切
fn bounds_match(state: &EditorState) {
    for annotation in &state.annotations {
        if let AnnotationShape::Magnifier { magnifier } = annotation.shape {
            assert_eq!(annotation.bbox, magnifier.bounds().unwrap());
        }
    }
}

/// 【标注多选绘制测试】【动态缓存】边缘圆形和矩形改宽、共同旋转、非等比缩放后，分数条带重绘与提交缓存必须逐像素一致
#[test]
fn edge_magnifier_group_redraws_and_commit_keep_current_bounds_and_pixels() {
    let values = super::annotation_collection_reference::reference();
    for sample in values["width"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|sample| sample["scene"].as_u64().unwrap() >= 4 && sample["requested"] == 24)
    {
        for scale in [1.0, 1.5, 2.0] {
            let mut state = prepared(&sample["after"], scale);
            whole(&mut state);
            for handle in [
                SelectionHandle::Rotate,
                SelectionHandle::BottomRight,
                SelectionHandle::Move,
            ] {
                let bounds = state.collection_bounds().unwrap();
                state.input.pointer.global = (
                    f64::from((bounds.left() + bounds.right()) / 2.0),
                    f64::from(bounds.top() - 26.0),
                );
                state.begin_collection_drag(handle);
                state.update_collection_drag((
                    f64::from(bounds.right() - 18.625),
                    f64::from(bounds.bottom() - 13.375),
                ));
                bounds_match(&state);
                let request = frame(&state);
                let mut dimmed_for = state.selection.zone;
                render_monitor(&mut state, 0, &request, None, &mut dimmed_for);
                let expected = state.canvas[0].clone();
                // 1. 【标注多选绘制测试】【局部重绘】重复裁切双框、连接线与透明遮挡对象，固定像素相位保持相同
                for x in [3.375, 27.625, 180.25, 295.375, 535.625, 596.5] {
                    let dirty = Rect::from_xywh(x, 0.0, 13.625, 460.375).unwrap();
                    for _ in 0..3 {
                        render_monitor(&mut state, 0, &request, Some(dirty), &mut dimmed_for);
                    }
                    same(
                        &state.canvas[0],
                        &expected,
                        &format!(
                            "scene={} scale={scale} handle={handle:?} x={x}",
                            sample["scene"]
                        ),
                    );
                }
                // 2. 【标注多选绘制测试】【持久提交】结束共同手势后恢复缓存，正文和未选中对象保持原始层次
                state.finish_collection_drag();
                same(
                    &whole(&mut state),
                    &expected,
                    &format!("scale={scale} committed {handle:?}"),
                );
            }
        }
    }
}
