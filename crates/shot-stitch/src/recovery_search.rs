//! 【长图拼接】【历史候选】按少量正文行筛选任意像素位置，完整校验次数不随累计长图高度增长

use crate::{
    profile::{BAND_COUNT, Signature, Weights},
    types::NO_MATCH,
};

const PROBES: usize = 12;
const CANDIDATES: usize = 24;

/// 【长图拼接】【采样行筛选】按 frame、top、bottom 和 weights 返回分散的有纹理正文行，不选择固定顶底栏
fn probes(
    frame: &Signature,
    top: usize,
    bottom: usize,
    weights: &Weights,
    count: usize,
) -> Vec<usize> {
    let end = frame.len().saturating_sub(bottom);
    let mut rows = Vec::with_capacity(count);
    // 1. 【长图拼接】【正文分段】分散采样避免候选只命中一段重复空白
    for section in 0..count {
        let start = top + end.saturating_sub(top) * section / count;
        let limit = top + end.saturating_sub(top) * (section + 1) / count;
        let row = (start.max(1)..limit)
            .map(|row| {
                let detail = weights
                    .iter()
                    .enumerate()
                    .map(|(band, weight)| weight * (frame[row][band] - frame[row - 1][band]).abs())
                    .sum::<f32>();
                (row, detail)
            })
            .max_by(|left, right| left.1.total_cmp(&right.1));
        if let Some((row, _)) = row {
            rows.push(row);
        }
    }
    rows
}

/// 【长图拼接】【行差异】按 first、second 和 weights 返回活动带平均误差，保留既有单带异常剔除规则
fn difference(first: &[f32; BAND_COUNT], second: &[f32; BAND_COUNT], weights: &Weights) -> f32 {
    let (mut sum, mut count, mut worst) = (0.0_f32, 0, 0.0_f32);
    for band in 0..BAND_COUNT {
        if weights[band] <= 0.0 {
            continue;
        }
        let value = (first[band] - second[band]).abs();
        sum += value;
        worst = worst.max(value);
        count += 1;
    }
    if count >= 4 {
        sum -= worst;
        count -= 1;
    }
    if count == 0 {
        NO_MATCH
    } else {
        sum / count as f32
    }
}

/// 【长图拼接】【位置搜索】按 full、frame、prediction、weights、excluded、accept 和边缘范围返回最多二十四个候选，range 为空时搜索完整历史
pub(crate) fn candidates(
    full: &Signature,
    frame: &Signature,
    prediction: i32,
    weights: &Weights,
    excluded: (i32, i32),
    accept: f32,
    range: Option<(i32, i32)>,
) -> Vec<i32> {
    let rows = probes(
        frame,
        excluded.0.max(0) as usize,
        excluded.1.max(0) as usize,
        weights,
        if range.is_some() { PROBES * 4 } else { PROBES },
    );
    if rows.len() < 8 {
        return Vec::new();
    }
    let mut best: Vec<(i32, f32)> = Vec::with_capacity(CANDIDATES);
    let threshold = (accept * 2.0).max(0.01);
    // 1. 【长图拼接】【逐像素筛选】廉价分散行比较覆盖全部位置，不再跳过八像素网格之间的真匹配
    let (start, end) = range.unwrap_or((0, full.len() as i32 - frame.len() as i32));
    for position in start..=end {
        let included =
            |row: usize| (0..full.len() as i64).contains(&(i64::from(position) + row as i64));
        let compared = rows.iter().filter(|&&row| included(row)).count();
        if compared < 8 {
            continue;
        }
        let (mut sum, mut misses) = (0.0, 0);
        for &row in &rows {
            if !included(row) {
                continue;
            }
            let value = difference(
                &full[(i64::from(position) + row as i64) as usize],
                &frame[row],
                weights,
            );
            sum += value.min(threshold * 4.0);
            misses += usize::from(value > threshold);
            if misses > compared / 4 {
                break;
            }
        }
        if misses > compared / 4 {
            continue;
        }
        let item = (position, sum / compared as f32);
        let order = |left: &(i32, f32), right: &(i32, f32)| {
            left.1.total_cmp(&right.1).then_with(|| {
                (left.0 - prediction)
                    .abs()
                    .cmp(&(right.0 - prediction).abs())
            })
        };
        let index = best.partition_point(|current| order(current, &item).is_le());
        // 2. 【长图拼接】【候选上限】仅保存最有价值的位置，限制后续整帧评分和接缝核验次数
        if index < CANDIDATES {
            best.insert(index, item);
            best.truncate(CANDIDATES);
        }
    }
    best.into_iter().map(|candidate| candidate.0).collect()
}
