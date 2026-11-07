//! 【区域识别】【原图精修】缩图候选映射和原始颜色边界搜索

use super::{
    PixelRect,
    image::{Rgb, gray::difference},
};

/// 【区域识别】【尺寸映射】按 rect、analysis 和 original 映射到原始像素，返回整数四舍五入边界
pub(super) fn scale(
    rect: PixelRect,
    analysis: (usize, usize),
    original: (usize, usize),
) -> PixelRect {
    let convert = |value: i32, from: usize, to: usize| {
        ((i64::from(value) * to as i64 + from as i64 / 2) / from as i64) as i32
    };
    PixelRect {
        left: convert(rect.left, analysis.0, original.0),
        top: convert(rect.top, analysis.1, original.1),
        right: convert(rect.right, analysis.0, original.0),
        bottom: convert(rect.bottom, analysis.1, original.1),
    }
}

/// 【区域识别】【单边精修】按 image、original、start、end、vertical、radius、threshold 搜索原图色差，返回可靠边界坐标
fn side(
    image: &Rgb<'_>,
    original: i32,
    start: i32,
    end: i32,
    vertical: bool,
    radius: i32,
    threshold: u8,
) -> i32 {
    let limit = if vertical { image.width } else { image.height } as i32;
    if original <= 1 || original >= limit - 1 {
        return original;
    }
    let margin = ((end - start) / 10).clamp(1, 24);
    let start = (start + margin).max(0);
    let end = (end - margin).min(if vertical { image.height } else { image.width } as i32);
    if end <= start {
        return original;
    }
    let step = ((end - start) / 128).max(1) as usize;
    let mut best = original;
    let mut best_score = 0.0;
    for position in (original - radius).max(1)..=(original + radius).min(limit - 1) {
        let mut count = 0;
        let mut strength = 0.0f32;
        let mut samples = 0;
        for along in (start..end).step_by(step) {
            let (a, b) = if vertical {
                (
                    image.at(position as usize - 1, along as usize),
                    image.at(position as usize, along as usize),
                )
            } else {
                (
                    image.at(along as usize, position as usize - 1),
                    image.at(along as usize, position as usize),
                )
            };
            let diff = difference(a, b);
            count += i32::from(diff >= threshold.min(8));
            strength += f32::from(diff.min(48)) / 48.0;
            samples += 1;
        }
        let coverage = count as f32 / samples.max(1) as f32;
        let score = coverage * 0.85 + strength / samples.max(1) as f32 * 0.15
            - (position - original).abs() as f32 * 0.008;
        if coverage >= 0.55 && score > best_score {
            best_score = score;
            best = position;
        }
    }
    best
}

/// 【区域识别】【四边精修】按 image、rect、radius 和 threshold 先水平定位再垂直定位，返回图像内有效矩形
pub(super) fn refine(image: &Rgb<'_>, rect: PixelRect, radius: i32, threshold: u8) -> PixelRect {
    // 1. 【区域识别】【左右边界】以原始纵向区间定位左右边，保留移动距离惩罚
    let left = side(
        image,
        rect.left,
        rect.top,
        rect.bottom,
        true,
        radius,
        threshold,
    );
    let right = side(
        image,
        rect.right,
        rect.top,
        rect.bottom,
        true,
        radius,
        threshold,
    );
    // 2. 【区域识别】【上下边界】使用已精修水平区间定位上下边，过小结果保持原始候选
    let top = side(image, rect.top, left, right, false, radius, threshold);
    let bottom = side(image, rect.bottom, left, right, false, radius, threshold);
    if right - left < 8 || bottom - top < 8 {
        return rect;
    }
    PixelRect {
        left,
        top,
        right,
        bottom,
    }
    .intersect(PixelRect::new(
        0,
        0,
        image.width as i32,
        image.height as i32,
    ))
    .unwrap_or(rect)
}
