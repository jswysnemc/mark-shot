//! 【滚动截图】【遮挡证据】按正文坐标比较重叠行，局部阴影不会被整行平均稀释

const BLOCKS: usize = 8;
const SAMPLES_PER_BLOCK: usize = 32;
const CLEAN_DIFFERENCE: f32 = 1.0;

/// 【滚动截图】【连续采样校验】按 offset 和 excluded 比较相邻原始帧，整段为均衡误差时返回匹配行数
pub(crate) fn resampled_overlap(
    previous: &shot_image::Image,
    current: &shot_image::Image,
    offset: i32,
    span: (i32, i32),
    excluded: (i32, i32),
) -> i32 {
    let start = excluded.0.max(offset + excluded.0).max(0);
    let end = (previous.height() - excluded.1).min(offset + current.height() - excluded.1);
    let mut textured = 0;
    for vertical in start..end {
        let evidence = compare(previous.row(vertical), current.row(vertical - offset), span);
        if !evidence.resampled {
            return 0;
        }
        textured += i32::from(evidence.detail > 1.0);
    }
    if textured >= 8 {
        (end - start).max(0)
    } else {
        0
    }
}

pub(crate) struct Evidence {
    pub clean: bool,
    pub detail: f32,
    pub difference: f32,
    pub resampled: bool,
}

/// 【滚动截图】【重叠行比较】比较 previous、current 在 span 内的颜色差异，返回清晰匹配和边缘强度
pub(crate) fn compare(previous: &[u32], current: &[u32], span: (i32, i32)) -> Evidence {
    let start = span.0.max(0) as usize;
    let end = (span.0 + span.1).max(0) as usize;
    let end = end.min(previous.len()).min(current.len());
    if start >= end {
        return Evidence {
            clean: false,
            detail: f32::MAX,
            difference: f32::MAX,
            resampled: false,
        };
    }
    let mut maximum = 0.0_f32;
    let mut detail = 0.0_f32;
    let mut samples = 0;
    let mut bias = 0.0_f32;
    for block in 0..BLOCKS {
        let left = start + (end - start) * block / BLOCKS;
        let right = start + (end - start) * (block + 1) / BLOCKS;
        let step = (right - left).div_ceil(SAMPLES_PER_BLOCK).max(1);
        let mut difference = 0.0;
        let mut count = 0;
        let mut signed = [0.0_f32; 3];
        for horizontal in (left..right).step_by(step) {
            for (channel, shift) in [0, 8, 16].into_iter().enumerate() {
                let old = ((previous[horizontal] >> shift) & 255) as f32;
                let new = ((current[horizontal] >> shift) & 255) as f32;
                difference += (old - new).abs();
                signed[channel] += old - new;
                if horizontal > left {
                    let neighbor = ((current[horizontal - 1] >> shift) & 255) as f32;
                    detail += (new - neighbor).abs();
                }
                count += 1;
            }
        }
        if count > 0 {
            maximum = maximum.max(difference / count as f32);
            bias = signed
                .into_iter()
                .map(|sum| sum.abs() * 3.0 / count as f32)
                .fold(bias, f32::max);
            samples += count;
        }
    }
    Evidence {
        clean: maximum <= CLEAN_DIFFERENCE,
        detail: detail / samples.max(1) as f32,
        difference: maximum,
        resampled: maximum <= 12.0 && bias <= 3.0,
    }
}
