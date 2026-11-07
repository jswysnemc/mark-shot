//! 【区域识别】【单次分析】方向边缘提案、评分和有限原图精修

use super::{
    Candidate, Source,
    candidate::merge,
    edges,
    image::{Gray, Rgb, morphology, resize},
    proposals, refinement, scoring,
};

pub(super) const MAX_DIMENSION: usize = 1280;
const MAX_PROPOSALS: usize = 4096;
const MAX_REFINED: usize = 512;
pub(super) const MAX_CANDIDATES: usize = 512;

/// 【区域识别】【分析尺寸】按 width、height 计算不超过一千二百八十像素的分析大小，返回至少三像素短边
pub(super) fn size(width: usize, height: usize) -> (usize, usize) {
    let longest = width.max(height);
    if longest <= MAX_DIMENSION {
        return (width, height);
    }
    let scale = MAX_DIMENSION as f64 / longest as f64;
    (
        (width as f64 * scale).round().max(3.0) as usize,
        (height as f64 * scale).round().max(3.0) as usize,
    )
}

/// 【区域识别】【完整分析】按 source 的正常颜色执行一次区域分析，返回已评分和精修的候选，不保存临时图像
pub(super) fn detect(source: &Rgb<'_>) -> Vec<Candidate> {
    let (width, height) = (source.width, source.height);
    if width < 3 || height < 3 {
        return Vec::new();
    }
    let analysis = size(width, height);
    let image = resize::downscale(source, analysis.0, analysis.1);
    let field = edges::build(&image);
    let closed = morphology::close(&field.combined);
    let mut proposals = proposals::contours(&closed);
    proposals.extend(proposals::component_regions(&closed, false));
    proposals.extend(proposals::lines(&field));
    // 1. 【区域识别】【背景提案】方向边膨胀后反相，四邻域背景提供面板和分隔区域
    let barriers = morphology::dilate(&field.combined);
    let mut background = Gray::new(analysis.0, analysis.1);
    for (value, barrier) in background.pixels.iter_mut().zip(&barriers.pixels) {
        *value = if *barrier == 0 { 255 } else { 0 };
    }
    proposals.extend(proposals::component_regions(&background, true));
    let scale = width as f32 / analysis.0 as f32;
    let min_width = (12.0 / scale).ceil().max(5.0) as i32;
    let min_height = (12.0 / scale).ceil().max(5.0) as i32;
    let full_area = (analysis.0 * analysis.1) as i64;
    proposals.retain(|proposal| {
        proposal.rect.width() >= min_width
            && proposal.rect.height() >= min_height
            && proposal.rect.area() < full_area * 995 / 1000
    });
    // 2. 【区域识别】【提案预算】小控件与大面板各保留一半预算，不只保留单一尺度
    proposals.sort_by_key(|proposal| proposal.rect.area());
    if proposals.len() > MAX_PROPOSALS {
        let keep = MAX_PROPOSALS / 2;
        proposals.drain(keep..proposals.len() - keep);
    }
    let mut candidates = Vec::new();
    let merge_tolerance = scale.ceil() as i32 + 1;
    for proposal in proposals {
        let rect = proposal.rect;
        let sides = scoring::sides(&field, rect);
        let mut sorted = sides;
        sorted.sort_by(f32::total_cmp);
        let support = (sides[0] + sides[1] + sides[2] + sides[3]) / 4.0;
        if sorted[1] < 0.48 || sorted[0] < 0.18 || support < 0.58 {
            continue;
        }
        if proposal.source == Source::Lines && (sorted[0] < 0.70 || support < 0.85) {
            continue;
        }
        let density = scoring::density(&field.combined, rect);
        let inset = scoring::inset(&field, rect);
        let compact = rect.width().min(rect.height()) as f32 * scale < 32.0;
        if compact && (density > 0.30 || density > 0.12 && inset > 0.60) {
            continue;
        }
        let texture_penalty = if compact {
            density * 0.30 + inset * 0.18
        } else {
            density * 0.08
        };
        let candidate = Candidate {
            rect: refinement::scale(rect, analysis, (width, height)),
            quality: (support * 0.65 + sorted[0] * 0.25 + 0.10 - texture_penalty).clamp(0.0, 1.0),
            sides,
            interior_edges: density,
            sources: vec![proposal.source],
        };
        merge(&mut candidates, candidate, merge_tolerance);
    }
    // 3. 【区域识别】【精修预算】只精修最多五百一十二个高质量区域，再按两像素边界容差合并
    candidates.sort_by(|a, b| b.quality.total_cmp(&a.quality));
    candidates.truncate(MAX_REFINED);
    let radius = ((scale.ceil() as i32) * 3).clamp(3, 16);
    let mut refined = Vec::new();
    for mut candidate in candidates {
        candidate.rect = refinement::refine(source, candidate.rect, radius, field.threshold);
        merge(&mut refined, candidate, 2);
    }
    refined
}
