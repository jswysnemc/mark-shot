//! 【区域识别】【矩形提案】轮廓、前景背景连通域与方向线段配对

use super::{
    PixelRect, Source,
    edges::Field,
    image::{Gray, components},
};

pub(super) struct Proposal {
    pub rect: PixelRect,
    pub source: Source,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Segment {
    fixed: i32,
    start: i32,
    end: i32,
}

/// 【区域识别】【轮廓提案】按 edges 的八邻域细环外接框生成矩形，返回前景至少十六像素的提案
pub(super) fn contours(edges: &Gray) -> Vec<Proposal> {
    components::connected(edges, true)
        .into_iter()
        .filter(|component| {
            component.pixels >= 16 && component.rect.width() > 0 && component.rect.height() > 0
        })
        .map(|component| Proposal {
            rect: component.rect,
            source: Source::Contour,
        })
        .collect()
}

/// 【区域识别】【连通域提案】按 image 和 background 生成前景或背景区域，返回至少二十四像素的有效矩形
pub(super) fn component_regions(image: &Gray, background: bool) -> Vec<Proposal> {
    components::connected(image, !background)
        .into_iter()
        .filter(|component| {
            component.pixels >= 24
                && (!background
                    || f64::from(component.pixels) / component.rect.area().max(1) as f64 >= 0.72)
        })
        .map(|component| Proposal {
            rect: component.rect,
            source: if background {
                Source::Background
            } else {
                Source::Component
            },
        })
        .collect()
}

/// 【区域识别】【长线提取】按 edges、vertical 扫描方向边，返回去重后最多一百九十二条线段，允许中途三个像素空隙
fn segments(edges: &Gray, vertical: bool) -> Vec<Segment> {
    let (across, along) = if vertical {
        (edges.width, edges.height)
    } else {
        (edges.height, edges.width)
    };
    let mut segments = Vec::new();
    // 1. 【区域识别】【线段扫描】保留连续位置和最后命中点，过滤不足二十五个连续位置的短边
    for fixed in 0..across {
        let mut start = None;
        let mut last = 0;
        for position in 0..=along {
            let hit = position < along
                && if vertical {
                    edges.at(fixed, position) != 0
                } else {
                    edges.at(position, fixed) != 0
                };
            if hit {
                if start.is_none() {
                    start = Some(position);
                }
                last = position;
            } else if start.is_some()
                && (position == along || position - last > 3)
                && let Some(start) = start.take().filter(|start| last - *start >= 24)
            {
                segments.push(Segment {
                    fixed: fixed as i32,
                    start: start as i32,
                    end: last as i32 + 1,
                });
            }
        }
    }
    // 2. 【区域识别】【线段去重】优先完整长边，相近边只保留一个代表，不扩大配对预算
    segments.sort_by_key(|segment| std::cmp::Reverse(segment.end - segment.start));
    let mut unique: Vec<Segment> = Vec::new();
    for segment in segments {
        if unique.iter().any(|other| {
            (segment.fixed - other.fixed).abs() <= 2
                && (segment.start - other.start).abs() <= 5
                && (segment.end - other.end).abs() <= 5
        }) {
            continue;
        }
        unique.push(segment);
        if unique.len() >= 192 {
            break;
        }
    }
    unique
}

/// 【区域识别】【线段容差】按 first、second 的长度返回起止点允许误差，范围为四至十六像素
fn tolerance(first: Segment, second: Segment) -> i32 {
    ((first.end - first.start).min(second.end - second.start) / 12).clamp(4, 16)
}

/// 【区域识别】【直线提案】按 field 的平行方向边配对矩形，返回近端两组及最远端提案
pub(super) fn lines(field: &Field) -> Vec<Proposal> {
    let mut proposals = Vec::new();
    for vertical in [true, false] {
        let mut lines = segments(
            if vertical {
                &field.vertical
            } else {
                &field.horizontal
            },
            vertical,
        );
        lines.sort_by_key(|line| line.fixed);
        for (index, first) in lines.iter().copied().enumerate() {
            let aligned: Vec<Segment> = lines[index + 1..]
                .iter()
                .copied()
                .filter(|second| {
                    let tolerance = tolerance(first, *second);
                    (first.fixed - second.fixed).abs() >= 8
                        && (first.start - second.start).abs() <= tolerance
                        && (first.end - second.end).abs() <= tolerance
                })
                .collect();
            let mut pairs: Vec<Segment> = aligned.iter().take(2).copied().collect();
            if let Some(last) = aligned.last().copied().filter(|last| !pairs.contains(last)) {
                pairs.push(last);
            }
            for second in pairs {
                let tolerance = tolerance(first, second);
                if (first.fixed - second.fixed).abs() < 8
                    || (first.start - second.start).abs() > tolerance
                    || (first.end - second.end).abs() > tolerance
                {
                    continue;
                }
                let low = first.fixed.min(second.fixed);
                let high = first.fixed.max(second.fixed) + 1;
                let start = first.start.min(second.start);
                let end = first.end.max(second.end);
                let rect = if vertical {
                    PixelRect {
                        left: low,
                        top: start,
                        right: high,
                        bottom: end,
                    }
                } else {
                    PixelRect {
                        left: start,
                        top: low,
                        right: end,
                        bottom: high,
                    }
                };
                proposals.push(Proposal {
                    rect,
                    source: Source::Lines,
                });
            }
        }
    }
    proposals
}
