//! 【尺子】【说明缓存】按测量变化更新全部说明，只通知新旧说明与变化刻度区域

use crate::editor::{DamageZone, EditorState};
use crate::features::ruler::labels::prepare_all;
use crate::features::ruler::state::LabelSource;
use crate::tools::drawing_space::DrawingSpace;
use crate::types::AnnotationShape;
use crate::utils::get_overlapping_monitors;

/// 【尺子】【说明刷新】按 state 的持久标注、草稿和选区更新说明缓存；dirty_mask 返回重绘显示器，无返回值
pub fn refresh(state: &mut EditorState, dirty_mask: &mut u32) {
    let sources: Vec<_> = state
        .annotations
        .iter()
        .chain(state.pending.iter())
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
        DrawingSpace::from_editor(state).and_then(|space| {
            state
                .selection
                .zone
                .unwrap_or(space.bounds)
                .intersect(&space.bounds)
        })
    };
    if sources == state.ruler.label_sources && bounds == state.ruler.label_bounds {
        return;
    }
    let labels = bounds.map_or_else(Vec::new, |bounds| {
        prepare_all(
            sources
                .iter()
                .map(|source| (source.measurement, source.width)),
            bounds,
            &mut state.text.font_system,
        )
    });
    // 1. 【尺子】【说明刷新】避让布局可能移动其他说明，全部旧位置和新位置都参与局部重绘
    for label in state.ruler.labels.iter().chain(labels.iter()) {
        state.damage_rects.push(DamageZone::Global(label.rect));
        *dirty_mask |= get_overlapping_monitors(&label.rect, &state.placements);
    }
    // 2. 【尺子】【说明刷新】允许范围改变后重画刻度数字，但不重建普通标注或分配新截图层
    if bounds != state.ruler.label_bounds {
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
    state.ruler.labels = labels;
}
