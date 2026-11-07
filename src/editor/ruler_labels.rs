//! 【尺子】【说明缓存】按测量变化更新全部说明，只通知新旧说明与变化刻度区域

use crate::editor::{DamageZone, EditorState};
use crate::features::ruler::labels::prepare_in_projection;
use crate::features::ruler::state::LabelSource;
use crate::types::AnnotationShape;
use crate::utils::get_overlapping_monitors;

/// 【尺子】【说明刷新】按 state 的持久标注、草稿和选区更新说明缓存；dirty_mask 返回重绘显示器，无返回值
pub fn refresh(state: &mut EditorState, dirty_mask: &mut u32) {
    // 1. 【尺子】【实时顺序】调整中的标注沿用原始位置，新建草稿才放到持久标注之后
    let mut annotations: Vec<_> = state
        .annotations
        .iter()
        .map(|annotation| {
            state
                .pending
                .as_ref()
                .filter(|pending| pending.id == annotation.id)
                .unwrap_or(annotation)
        })
        .collect();
    if let Some(pending) = state.pending.as_ref()
        && !state
            .annotations
            .iter()
            .any(|annotation| annotation.id == pending.id)
    {
        annotations.insert(state.dragged_index().unwrap_or(annotations.len()), pending);
    }
    let sources: Vec<_> = annotations
        .into_iter()
        .filter_map(|annotation| {
            if let AnnotationShape::Ruler { measurement } = annotation.shape {
                Some(LabelSource {
                    id: annotation.id,
                    measurement,
                    width: annotation.stroke_width,
                })
            } else {
                None
            }
        })
        .collect();
    let bounds = if sources.is_empty() {
        None
    } else {
        state.ruler_allowed_bounds()
    };
    let window = state.hit_window_units();
    let projection = state.canvas_transform();
    if sources == state.ruler.label_sources
        && bounds == state.ruler.label_bounds
        && window == state.ruler.label_window
        && projection == state.ruler.label_projection
    {
        return;
    }
    let labels = bounds.map_or_else(Vec::new, |bounds| {
        prepare_in_projection(
            sources
                .iter()
                .map(|source| (source.measurement, source.width)),
            bounds,
            projection,
        )
    });
    // 2. 【尺子】【说明刷新】避让布局可能移动其他说明，全部旧位置和新位置都参与局部重绘
    for label in state.ruler.labels.iter().chain(labels.iter()) {
        state.damage_rects.push(DamageZone::Global(label.rect));
        *dirty_mask |= get_overlapping_monitors(&label.rect, &state.placements);
    }
    // 3. 【尺子】【说明刷新】允许范围改变后重画刻度数字，但不重建普通标注或分配新截图层
    if bounds != state.ruler.label_bounds || projection != state.ruler.label_projection {
        for annotation in state
            .annotations
            .iter()
            .filter(|annotation| matches!(annotation.shape, AnnotationShape::Ruler { .. }))
        {
            let area = annotation.damage_bbox(false);
            state.layer_damage_rects.push(area);
            state.damage_rects.push(DamageZone::Global(area));
            *dirty_mask |= get_overlapping_monitors(&area, &state.placements);
        }
    }
    state.ruler.label_sources = sources;
    state.ruler.label_bounds = bounds;
    state.ruler.label_window = window;
    state.ruler.label_projection = projection;
    state.ruler.labels = labels;
}
