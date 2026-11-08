//! 【滚动截图】【连接位置】在连续清晰正文中连接，末帧真实边缘留在原存储中供后续替换

use crate::{Edge, Stitcher, edge_occlusion};
use shot_image::Image;

pub(crate) struct Replacement {
    pub rows: i32,
    pub trim: i32,
    pub matching_rows: i32,
}

/// 【滚动截图】【位移复核】按 frame、cols 验证 offset，正文证据不足时校验历史及中心候选，返回可信位移和评分
pub(crate) fn refine(
    stitcher: &Stitcher,
    frame: &Image,
    cols: &crate::profile::Signature,
    offset: i32,
    confidence: f32,
) -> (i32, f32) {
    let rows = |candidate| {
        find(stitcher, frame, stitcher.anchor + candidate, Edge::None)
            .map_or(0, |seam| seam.matching_rows)
    };
    let mut best_rows = rows(offset);
    if best_rows >= stitcher.config.min_overlap {
        return (offset, confidence);
    }
    let mut best = (offset, confidence);
    for candidate in [stitcher.last_offset, stitcher.find_inner_offset(cols).0] {
        if candidate == best.0 || !stitcher.reachable(stitcher.anchor + candidate, frame.height()) {
            continue;
        }
        let matching = rows(candidate);
        if matching >= stitcher.config.min_overlap && matching > best_rows {
            best = (candidate, 0.0);
            best_rows = matching;
        }
    }
    best
}

/// 【滚动截图】【重叠选缝】按 position、edge 对齐 stitcher 原图与 frame，返回原像素替换范围及匹配行数
pub(crate) fn find(
    stitcher: &Stitcher,
    frame: &Image,
    position: i32,
    edge: Edge,
) -> Option<Replacement> {
    let long = &stitcher.long;
    let span = stitcher.motion.moving_span(frame.width());
    let excluded = (stitcher.fixed.top(), stitcher.fixed.bottom());
    // 1. 【滚动截图】【边缘排除】两帧固定区域都不参与连接，坐标始终以已有长图正文为准
    let start = excluded.0.max(position + excluded.0).max(0);
    let end = (long.height() - excluded.1).min(position + frame.height() - excluded.1);
    let required = stitcher.config.min_overlap.clamp(8, 32);
    let mut run_start = start;
    let mut textured = 0;
    let mut best = start..start;
    for vertical in start..end {
        let evidence =
            edge_occlusion::compare(long.row(vertical), frame.row(vertical - position), span);
        if evidence.clean {
            let frame_row = vertical - position;
            if evidence.detail > 1.0
                || (frame_row > 0
                    && edge_occlusion::compare(
                        frame.row(frame_row - 1),
                        frame.row(frame_row),
                        span,
                    )
                    .difference
                        > 1.0)
            {
                textured += 1;
            }
            if textured >= 8 && vertical + 1 - run_start > best.end - best.start {
                best = run_start..vertical + 1;
            }
        } else {
            run_start = vertical + 1;
            textured = 0;
        }
    }
    if best.end - best.start < required {
        // 2. 【滚动截图】【分数缩放】整段只有均衡重采样误差时沿用原边界，避免覆盖已经清晰的旧正文
        let resampled_rows = stitcher.last_frame.as_ref().map_or(0, |previous| {
            edge_occlusion::resampled_overlap(
                previous,
                frame,
                position - stitcher.anchor,
                span,
                excluded,
            )
        });
        if resampled_rows >= stitcher.config.min_overlap {
            let trim = match edge {
                Edge::End => excluded.1,
                Edge::Start => excluded.0,
                Edge::None => 0,
            };
            let added = match edge {
                Edge::End => position + frame.height() - long.height(),
                Edge::Start => -position,
                Edge::None => 0,
            };
            return Some(Replacement {
                rows: added + trim,
                trim,
                matching_rows: resampled_rows,
            });
        }
        return None;
    }
    if edge == Edge::None {
        return Some(Replacement {
            rows: 0,
            trim: 0,
            matching_rows: best.end - best.start,
        });
    }
    // 2. 【滚动截图】【正文连接】仅在匹配区中部选低纹理行，远离阴影衰减边界并减少文字边缘切换
    let margin = (best.end - best.start) / 4;
    let middle = (best.start + best.end) / 2;
    let mut seam = middle;
    let mut detail = f32::MAX;
    for vertical in best.start + margin..best.end - margin {
        let evidence =
            edge_occlusion::compare(long.row(vertical), frame.row(vertical - position), span);
        if evidence.detail < detail
            || (evidence.detail == detail && (vertical - middle).abs() < (seam - middle).abs())
        {
            seam = vertical;
            detail = evidence.detail;
        }
    }
    // 3. 【滚动截图】【替换范围】新增行数减去替换行数等于真实位移，不按阴影高度删减正文
    match edge {
        Edge::End => Some(Replacement {
            rows: position + frame.height() - seam,
            trim: long.height() - seam,
            matching_rows: best.end - best.start,
        }),
        Edge::Start => Some(Replacement {
            rows: seam - position,
            trim: seam,
            matching_rows: best.end - best.start,
        }),
        Edge::None => Some(Replacement {
            rows: 0,
            trim: 0,
            matching_rows: best.end - best.start,
        }),
    }
}
