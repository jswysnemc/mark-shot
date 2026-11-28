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
        // 1. 【长图拼接】【像素优先】分散清晰正文已证实位移，阴影改变整段特征评分不能覆盖原始像素证据
        return (offset, 0.0);
    }
    // 2. 【长图拼接】【局部变色】平均亮度可能偏向另一行；先用未变的相邻纹理行提出位移，再核验整个重叠区
    if let Some(candidate) =
        crate::row_anchors::offset(&stitcher.last_cols, cols, stitcher.config.min_overlap)
        && candidate != offset
        && stitcher.reachable(stitcher.anchor + candidate, frame.height())
        && rows(candidate) >= stitcher.config.min_overlap
    {
        return (candidate, 0.0);
    }
    let mut best = (offset, confidence);
    let alternatives = stitcher.find_inner_offsets(cols);
    for candidate in [stitcher.last_offset, alternatives[0].0, alternatives[1].0] {
        if candidate == best.0 || !stitcher.reachable(stitcher.anchor + candidate, frame.height()) {
            continue;
        }
        let matching = rows(candidate);
        if matching >= stitcher.config.min_overlap && matching > best_rows {
            best = (candidate, 0.0);
            best_rows = matching;
        }
    }
    // 2. 【长图拼接】【恢复交接】签名看似通过但像素证据不足时，交给历史和边缘恢复，不能直接丢弃本帧
    if best_rows < stitcher.config.min_overlap {
        best.1 = crate::types::NO_MATCH;
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
    let excluded = (stitcher.fixed.top(), stitcher.fixed.bottom());
    let span = crate::content_span::refine(
        long,
        stitcher.last_frame.as_ref(),
        frame,
        position,
        (0, frame.width()),
        excluded,
    );
    // 1. 【滚动截图】【边缘排除】两帧固定区域都不参与连接，坐标始终以已有长图正文为准
    let start = excluded.0.max(position + excluded.0).max(0);
    let end = (long.height() - excluded.1).min(position + frame.height() - excluded.1);
    let required = stitcher.config.min_overlap.clamp(8, 32);
    let evidence = crate::overlap_evidence::scan(long, frame, position, span, start..end);
    let best = evidence.seam;
    let minimum = stitcher.config.min_overlap;
    let exact = best.len() >= required as usize
        && evidence.textured_rows >= 8
        && evidence.clean_rows >= minimum
        && (evidence.textured_run_rows >= minimum
            || crate::texture_consistency::matches(long, frame, position, span, start..end));
    let matching_rows = if exact {
        evidence.clean_rows
    } else if end - start >= minimum
        && best.len() >= 8
        && evidence.textured_rows >= 8
        && evidence.clean_rows >= (end - start) / 4
        && crate::hover_overlap::matches(long, frame, position, span, start..end)
    {
        // 2. 【滚动截图】【悬停底色】变色行只补充位移证据，接缝仍从未变色的连续段中选择
        end - start
    } else {
        // 2. 【滚动截图】【分数缩放】整段只有均衡重采样误差时沿用原边界，避免覆盖已经清晰的旧正文
        let resampled_rows = stitcher.last_frame.as_ref().map_or(0, |previous| {
            let rows = edge_occlusion::resampled_overlap(
                previous,
                frame,
                position - stitcher.anchor,
                span,
                excluded,
            );
            if rows > 0 {
                rows
            } else {
                // 3. 【长图拼接】【抗锯齿切换】浏览器停止合成滚动后可能切换文字渲染，需独立核验字形，沿用原图接缝
                crate::antialiasing::overlap(
                    previous,
                    frame,
                    position - stitcher.anchor,
                    span,
                    excluded,
                )
            }
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
    };
    if edge == Edge::None {
        return Some(Replacement {
            rows: 0,
            trim: 0,
            matching_rows,
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
            matching_rows,
        }),
        Edge::Start => Some(Replacement {
            rows: seam - position,
            trim: seam,
            matching_rows,
        }),
        Edge::None => Some(Replacement {
            rows: 0,
            trim: 0,
            matching_rows,
        }),
    }
}
