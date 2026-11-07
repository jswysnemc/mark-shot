//! 【标注多选对照测试】【生产样本】框选、共同变换和批量宽度逐项对照真实Qt完整函数结果

mod model;
use super::*;
use crate::{
    features::annotation_collection::geometry,
    types::{Annotation, AnnotationShape, SelectionHandle},
};
use serde_json::Value;
use tiny_skia::Rect;

/// 【标注多选对照测试】【参考读取】返回真实Qt采集结果，不以当前实现生成预期，无参数
pub(super) fn reference() -> Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/qt-annotation-collection/reference.json"
    )))
    .unwrap()
}

/// 【标注多选对照测试】【编辑器构造】按sample的before建立真实编辑器，选择顺序按生产反向编号，不模拟变换
pub(super) fn prepared(sample: &Value) -> EditorState {
    let mut state = editor();
    state.selected_tool = crate::tools::Tool::Pick;
    state.selection.zone = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    state.ui_hidden = true;
    state.annotations = sample["before"]
        .as_array()
        .unwrap()
        .iter()
        .map(model::decode)
        .collect();
    for index in 0..state.annotations.len() {
        crate::editor::annotation_drag::rebuild_annotation(&mut state, index);
    }
    let ids: Vec<_> = state
        .annotations
        .iter()
        .rev()
        .map(|annotation| annotation.id)
        .collect();
    state.select_annotation_ids(ids);
    state
}

/// 【标注多选对照测试】【误差检查】按actual、expected与context收集超过源图千分之一像素的差异，errors返回所有失败信息
fn compare(actual: &[f64], expected: &Value, context: &str, errors: &mut Vec<String>) {
    let expected: Vec<_> = expected
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_f64().unwrap())
        .collect();
    if actual.len() != expected.len()
        || actual
            .iter()
            .zip(&expected)
            .any(|(a, e)| (a - e).abs() > 0.001)
    {
        errors.push(format!(
            "{context}: actual={actual:?}, expected={expected:?}"
        ));
    }
}

/// 【标注多选对照测试】【完整成员】按annotation与expected比较实际矩形、定点、角度、宽度和可见范围，errors返回差异，无返回值
fn member(annotation: &Annotation, expected: &Value, context: &str, errors: &mut Vec<String>) {
    use AnnotationShape::*;
    let point = |p: (f32, f32)| serde_json::json!([p.0, p.1]);
    let rectangle = |first: (f32, f32), last: (f32, f32)| {
        [
            f64::from(first.0.min(last.0)),
            f64::from(first.1.min(last.1)),
            f64::from((last.0 - first.0).abs()),
            f64::from((last.1 - first.1).abs()),
        ]
    };
    let (points, rotation, width) = match &annotation.shape {
        Rectangle { start, end, .. } | Circle { start, end, .. } | Marker { start, end, .. } => {
            compare(
                &rectangle(*start, *end),
                &expected["rect"],
                &format!("{context} rect"),
                errors,
            );
            (None, annotation.rotation_degrees, annotation.stroke_width)
        }
        Redact { filter } => {
            compare(
                &rectangle(filter.start, filter.end),
                &expected["rect"],
                &format!("{context} rect"),
                errors,
            );
            (
                None,
                f64::from(filter.rotation.to_degrees()),
                filter.strength,
            )
        }
        Magnifier { magnifier } => {
            let lens = magnifier.lens_rect().unwrap();
            compare(
                &[f64::from(magnifier.pivot.0), f64::from(magnifier.pivot.1)],
                &expected["pivot"],
                &format!("{context} pivot"),
                errors,
            );
            compare(
                &[
                    f64::from(lens.x()),
                    f64::from(lens.y()),
                    f64::from(lens.width()),
                    f64::from(lens.height()),
                ],
                &expected["rect"],
                &format!("{context} lens"),
                errors,
            );
            (
                Some(serde_json::json!([point(magnifier.source)])),
                f64::from(magnifier.rotation.to_degrees()),
                annotation.stroke_width,
            )
        }
        Number { mark } => (
            Some(serde_json::json!([point(mark.tip), point(mark.bubble)])),
            f64::from(mark.rotation.to_degrees()),
            annotation.stroke_width,
        ),
        Ruler { measurement } => match measurement.geometry {
            crate::features::ruler::RulerGeometry::Length {
                start,
                end,
                rotation,
            } => {
                compare(
                    &rectangle(start, end),
                    &expected["rect"],
                    &format!("{context} ruler"),
                    errors,
                );
                (None, f64::from(rotation), annotation.stroke_width)
            }
            crate::features::ruler::RulerGeometry::Angle { points, .. } => (
                Some(serde_json::json!(points.map(point))),
                0.0,
                annotation.stroke_width,
            ),
        },
        Text {
            font_size, frame, ..
        } => {
            let stored = annotation.text_stored_bounds().unwrap();
            compare(
                &[
                    f64::from(stored.x()),
                    f64::from(stored.y()),
                    f64::from(stored.width()),
                    f64::from(stored.height()),
                ],
                &expected["rect"],
                &format!("{context} stored"),
                errors,
            );
            (
                Some(serde_json::json!([point(frame.anchor.unwrap())])),
                annotation.rotation_degrees,
                font_size - 19.0,
            )
        }
        Pen { points } | Highlighter { points, .. } => (
            Some(serde_json::json!(
                points.iter().copied().map(point).collect::<Vec<_>>()
            )),
            annotation.rotation_degrees,
            annotation.stroke_width,
        ),
        Arrow { line, .. } | Line { line, .. } => (
            Some(serde_json::json!(
                line.points().map(point).collect::<Vec<_>>()
            )),
            annotation.rotation_degrees,
            annotation.stroke_width,
        ),
        _ => unreachable!(),
    };
    if let Some(points) = points {
        for (index, actual) in points.as_array().unwrap().iter().enumerate() {
            let actual: Vec<_> = actual
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_f64().unwrap())
                .collect();
            compare(
                &actual,
                &expected["points"][index],
                &format!("{context} point={index}"),
                errors,
            );
        }
    }
    if (rotation - expected["rotation"].as_f64().unwrap()).abs() > 0.001 {
        errors.push(format!(
            "{context} angle={rotation}, expected={}",
            expected["rotation"]
        ));
    }
    if (f64::from(width) - expected["width"].as_f64().unwrap()).abs() > 0.001 {
        errors.push(format!(
            "{context} width={width}, expected={}",
            expected["width"]
        ));
    }
    let canvas = Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap();
    let bounds = geometry::visible_bounds(annotation, canvas)
        .unwrap_or(Rect::from_xywh(0.0, 0.0, 0.0, 0.0).unwrap());
    compare(
        &[
            f64::from(bounds.x()),
            f64::from(bounds.y()),
            f64::from(bounds.width()),
            f64::from(bounds.height()),
        ],
        &expected["bounds"],
        &format!("{context} visible"),
        errors,
    );
}

/// 【标注多选对照测试】【生产框选】全部混合工具可见边界与相交编号必须匹配原版，不按正文像素命中代替选框命中
#[test]
fn every_qt_marquee_uses_the_same_visible_members_and_joint_bounds() {
    let values = reference();
    let mut errors = Vec::new();
    for (index, sample) in values["selection"].as_array().unwrap().iter().enumerate() {
        let state = prepared(sample);
        let ids = geometry::in_rect(
            &state.annotations,
            model::rect(&sample["marquee"]),
            state.selection.zone.unwrap(),
            1.0,
        );
        assert_eq!(serde_json::json!(ids), sample["ids"], "sample={index}");
        let bounds = state.collection_bounds().unwrap();
        compare(
            &[
                f64::from(bounds.x()),
                f64::from(bounds.y()),
                f64::from(bounds.width()),
                f64::from(bounds.height()),
            ],
            &sample["bounds"],
            &format!("sample={index} joint"),
            &mut errors,
        );
        for (annotation, expected) in state
            .annotations
            .iter()
            .zip(sample["before"].as_array().unwrap())
        {
            member(
                annotation,
                expected,
                &format!("sample={index} id={}", annotation.id),
                &mut errors,
            );
        }
    }
    assert!(
        errors.is_empty(),
        "{} mismatches:\n{}",
        errors.len(),
        errors
            .iter()
            .take(24)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// 【标注多选对照测试】【生产变换】全部共同移动、缩放、增量旋转与角度尺专用路径匹配完整Qt函数，每例只形成一次历史
#[test]
fn every_qt_group_transform_preserves_member_storage_and_rotation() {
    let values = reference();
    let mut errors = Vec::new();
    for (index, sample) in values["transform"].as_array().unwrap().iter().enumerate() {
        let mut state = prepared(sample);
        transition(&mut state, sample);
        assert_eq!(
            state.undo_stack.len(),
            sample["history"].as_u64().unwrap() as usize,
            "sample={index}"
        );
        for (annotation, expected) in state
            .annotations
            .iter()
            .zip(sample["after"].as_array().unwrap())
        {
            member(
                annotation,
                expected,
                &format!("sample={index} id={}", annotation.id),
                &mut errors,
            );
        }
    }
    assert!(
        errors.is_empty(),
        "{} mismatches:\n{}",
        errors.len(),
        errors
            .iter()
            .take(32)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// 【标注多选对照测试】【手势执行】按sample在state执行完整群组手势，保留既有成员和历史，无返回值
fn transition(state: &mut EditorState, sample: &Value) {
    let start = model::point(&sample["start"]);
    state.input.pointer.global = (f64::from(start.0), f64::from(start.1));
    let handle = [
        SelectionHandle::Move,
        SelectionHandle::Rotate,
        SelectionHandle::TopLeft,
        SelectionHandle::Right,
        SelectionHandle::BottomRight,
    ][sample["handle"].as_u64().unwrap() as usize];
    state.input.ctrl = sample["aspect"].as_bool().unwrap();
    state.input.shift = sample["snap"].as_bool().unwrap();
    state.begin_collection_drag(handle);
    let current = model::point(&sample["current"]);
    state.update_collection_drag((f64::from(current.0), f64::from(current.1)));
    state.finish_collection_drag();
}

/// 【标注多选对照测试】【连续状态】连续旋转、缩放、移动和吸附必须复用实际上一步成员，历史与Qt逐次一致
#[test]
fn every_qt_sequence_uses_the_previous_committed_members() {
    let values = reference();
    let mut errors = Vec::new();
    let mut state = None;
    for sample in values["sequence"].as_array().unwrap() {
        if sample["step"] == 0 {
            state = Some(prepared(sample));
        }
        let state = state.as_mut().unwrap();
        transition(state, sample);
        assert_eq!(
            state.undo_stack.len(),
            sample["history"].as_u64().unwrap() as usize
        );
        for (annotation, expected) in state
            .annotations
            .iter()
            .zip(sample["after"].as_array().unwrap())
        {
            member(
                annotation,
                expected,
                &format!(
                    "scene={} step={} id={}",
                    sample["scene"], sample["step"], annotation.id
                ),
                &mut errors,
            );
        }
    }
    assert!(
        errors.is_empty(),
        "{} mismatches:\n{}",
        errors.len(),
        errors
            .iter()
            .take(32)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// 【标注多选对照测试】【颜色事务】快捷同色逐次捕获，属性同色首次捕获，关闭与撤销不额外追加历史，默认颜色按生产来源隔离
#[test]
fn every_qt_color_input_captures_the_same_history_and_defaults() {
    let values = reference();
    let mut state = None;
    let mut previous_property = None;
    let bytes = |color: Color| {
        let color = color.to_color_u8();
        serde_json::json!([color.red(), color.green(), color.blue(), color.alpha()])
    };
    for sample in values["color"].as_array().unwrap() {
        let property = sample["property"].as_bool().unwrap();
        if previous_property != Some(property) {
            let mut fresh = prepared(sample);
            fresh.tool_settings.color = Color::from_rgba8(220, 40, 50, 255);
            fresh.redo_stack.push(fresh.snapshot());
            if property {
                crate::app::panels::color::toggle(
                    &mut fresh,
                    crate::ui::color_popover::ColorTarget::Main,
                    &mut 0,
                );
            }
            state = Some(fresh);
            previous_property = Some(property);
        }
        let state = state.as_mut().unwrap();
        let requested = sample["requested"].as_array().unwrap();
        let color = Color::from_rgba8(
            requested[0].as_u64().unwrap() as u8,
            requested[1].as_u64().unwrap() as u8,
            requested[2].as_u64().unwrap() as u8,
            requested[3].as_u64().unwrap() as u8,
        );
        if property {
            crate::editor::color_selection::apply_color_selection(state, color, false, &mut 0);
        } else {
            crate::editor::color_selection::apply_quick_color(state, color, &mut 0);
        }
        assert!(state.redo_stack.is_empty());
        assert_eq!(
            state.undo_stack.len(),
            sample["history"].as_u64().unwrap() as usize
        );
        assert_eq!(bytes(state.tool_settings.color), sample["default"]);
        for (annotation, expected) in state
            .annotations
            .iter()
            .zip(sample["after"].as_array().unwrap())
        {
            assert_eq!(bytes(annotation.color), expected["color"]);
        }
    }
    let state = state.as_mut().unwrap();
    crate::app::panels::color::close_color_popover(state, &mut 0);
    assert_eq!(state.undo_stack.len(), 1);
    key(state, "Ctrl+Z", &mut 0);
    assert!(state.undo_stack.is_empty());
    assert!(
        state
            .annotations
            .iter()
            .all(|annotation| annotation.color == Color::from_rgba8(30, 40, 50, 255))
    );
}

/// 【标注多选对照测试】【单项骨架】共同旋转后通过真实指针编辑端点和控制点，原图边缘限制、本地定点与角度匹配完整Qt函数
#[test]
fn every_qt_line_anchor_edit_matches_after_group_rotation() {
    let values = reference();
    let mut errors = Vec::new();
    for (index, sample) in values["line_edit"].as_array().unwrap().iter().enumerate() {
        let mut state = prepared(sample);
        let start = model::point(&sample["start"]);
        pointer(&mut state, f64::from(start.0), f64::from(start.1), &mut 0);
        super::super::handle_pointer_button(
            &mut state,
            crate::types::MouseButton::Left,
            true,
            &mut 0,
        );
        assert_eq!(
            state.ann_drag.as_ref().unwrap().handle,
            SelectionHandle::LinePoint(sample["point_index"].as_u64().unwrap() as usize)
        );
        let current = model::point(&sample["current"]);
        pointer(
            &mut state,
            f64::from(current.0),
            f64::from(current.1),
            &mut 0,
        );
        super::super::handle_pointer_button(
            &mut state,
            crate::types::MouseButton::Left,
            false,
            &mut 0,
        );
        assert_eq!(state.undo_stack.len(), 1);
        member(
            &state.annotations[0],
            &sample["after"][0],
            &format!("line_edit={index}"),
            &mut errors,
        );
    }
    assert!(
        errors.is_empty(),
        "{} mismatches:\n{}",
        errors.len(),
        errors.join("\n")
    );
}

/// 【标注多选对照测试】【生产宽度】文字偏移、普通二十四、荧光笔四十八与序号七十二上限匹配生产批量setter
#[test]
fn every_qt_bulk_width_uses_the_member_specific_semantics() {
    let values = reference();
    let mut errors = Vec::new();
    for (index, sample) in values["width"].as_array().unwrap().iter().enumerate() {
        let mut state = prepared(sample);
        let changed = crate::editor::annotation_collection::properties::set_width(
            &mut state,
            sample["requested"].as_i64().unwrap() as i32,
            true,
            &mut 0,
        );
        assert_eq!(changed, sample["changed"].as_bool().unwrap());
        assert_eq!(
            state.undo_stack.len(),
            sample["history"].as_u64().unwrap() as usize
        );
        if sample["scene"] == 2 {
            use crate::types::StrokeGroup;
            let defaults = &sample["defaults"];
            for (index, actual) in [
                state.tool_settings.stroke(StrokeGroup::Rectangle),
                state.tool_settings.stroke(StrokeGroup::Highlighter),
                state.tool_settings.font_size - 19.0,
                state.tool_settings.stroke(StrokeGroup::Number),
                state.tool_settings.strength,
                state.tool_settings.stroke(StrokeGroup::Ruler),
            ]
            .into_iter()
            .enumerate()
            {
                assert!(
                    (f64::from(actual) - defaults[index].as_f64().unwrap()).abs() < 0.001,
                    "sample={index} default={actual} expected={}",
                    defaults[index]
                );
            }
        }
        for (annotation, expected) in state
            .annotations
            .iter()
            .zip(sample["after"].as_array().unwrap())
        {
            member(
                annotation,
                expected,
                &format!("sample={index} id={}", annotation.id),
                &mut errors,
            );
        }
    }
    assert!(
        errors.is_empty(),
        "{} mismatches:\n{}",
        errors.len(),
        errors
            .iter()
            .take(24)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
