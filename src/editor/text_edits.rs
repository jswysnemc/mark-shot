//! 【文字属性】【样式修改】字体家族和字形独立于默认值，字号委托统一修改入口

use crate::{
    editor::{
        DamageZone, EditorState, annotation_drag::rebuild_annotation, dirty::apply_damage_rects,
        edits::active_annotation_idx,
    },
    types::AnnotationShape,
    ui::panel::{UiPanel, emit_panel_damage},
};

#[derive(Clone)]
pub enum Property {
    Points(f32),
    Family(String),
    Bold(bool),
    Italic(bool),
}

/// 【文字属性】【当前字号】返回 state 选中文字或工具默认使用的原始点字号，不返回预览像素
pub fn current_points(state: &EditorState) -> f32 {
    active_annotation_idx(state)
        .and_then(|index| match &state.annotations[index].shape {
            AnnotationShape::Text { font_size, .. } => Some(*font_size),
            _ => None,
        })
        .unwrap_or(state.tool_settings.font_size)
}

/// 【文字属性】【控件字号】按 points 更新 state 的控件数值，dirty 返回重绘范围；同值与到达边界不模拟原版未发出的数值变化信号
pub fn set_control_points(state: &mut EditorState, points: f32, dirty: &mut u32) {
    let points = crate::features::text::geometry::points(points);
    if points != current_points(state) {
        set(state, Property::Points(points), true, dirty);
    }
}

/// 【文字属性】【样式修改】将 property 应用到 state；record_undo 控制持久历史，dirty 返回损坏区域；字号另保留显式文档格式，其他同值不记录
pub fn set(state: &mut EditorState, mut property: Property, record_undo: bool, dirty: &mut u32) {
    if let Property::Points(points) = property {
        super::text_size::set(state, points, record_undo, dirty);
        super::text_session_format::sync(state, true, dirty);
        return;
    }
    if let Property::Family(family) = &mut property {
        *family = family.trim().to_owned();
        if family.is_empty() {
            return;
        }
    }
    let index = active_annotation_idx(state);
    if index.is_some_and(|index| {
        !matches!(state.annotations[index].shape, AnnotationShape::Text { .. })
    }) {
        return;
    }
    let changed = if let Some(index) = index {
        let AnnotationShape::Text {
            font_size,
            bold,
            italic,
            appearance,
            ..
        } = &state.annotations[index].shape
        else {
            unreachable!()
        };
        match &property {
            Property::Points(value) => font_size != value,
            Property::Family(value) => &appearance.family != value,
            Property::Bold(value) => bold != value,
            Property::Italic(value) => italic != value,
        }
    } else {
        let tools = &state.tool_settings;
        match &property {
            Property::Points(value) => tools.font_size != *value,
            Property::Family(value) => tools.text_family != *value,
            Property::Bold(value) => tools.bold != *value,
            Property::Italic(value) => tools.italic != *value,
        }
    };
    if !changed {
        return;
    }
    // 1. 【文字属性】【标注修改】字体家族和字形只修改选中对象
    if let Some(index) = index {
        let editor = crate::tools::text::ensure_text_editor(
            &state.annotations[index],
            &mut state.text.editors,
            &mut state.text.font_system,
        );
        let wrapped = cosmic_text::Edit::with_buffer(editor, |buffer| {
            buffer.wrap() != cosmic_text::Wrap::None
        });
        let stored_height = state.annotations[index]
            .text_stored_bounds()
            .map(|bounds| bounds.height());
        if record_undo {
            state.push_undo();
        }
        state.damage_rects.push(DamageZone::Global(
            state.annotations[index].damage_bbox(true),
        ));
        state
            .layer_damage_rects
            .push(state.annotations[index].damage_bbox(false));
        if let AnnotationShape::Text {
            font_size,
            bold,
            italic,
            appearance,
            frame,
            ..
        } = &mut state.annotations[index].shape
        {
            // 2. 【文字属性】【单行保持】原本没有换行的文字在字体变化后仍按自然宽度收框
            if !wrapped {
                frame.width = None;
                frame.height = None;
            } else {
                frame.height = frame.height.or(stored_height);
            }
            match property {
                Property::Points(value) => *font_size = value,
                Property::Family(value) => appearance.family = value,
                Property::Bold(value) => *bold = value,
                Property::Italic(value) => *italic = value,
            }
        }
        rebuild_annotation(state, index);
    } else {
        match property {
            Property::Points(value) => state.tool_settings.font_size = value,
            Property::Family(value) => state.tool_settings.text_family = value,
            Property::Bold(value) => state.tool_settings.bold = value,
            Property::Italic(value) => state.tool_settings.italic = value,
        }
    }
    // 3. 【文字属性】【属性重绘】样式改变后的字体、边界和属性数值统一进入损坏队列
    state.settings_panel.dirty = true;
    if let Some(rect) = state.settings_panel.rect() {
        emit_panel_damage(
            rect,
            state.settings_panel.monitor_idx,
            &mut state.damage_rects,
            dirty,
        );
    }
    apply_damage_rects(state, dirty);
}
