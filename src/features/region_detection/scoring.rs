//! 【区域识别】【候选评分】连续边界支持和内部纹理密度

use super::{PixelRect, edges::Field, image::Gray};

/// 【区域识别】【单边支持】按 edges、fixed、start、end 与 vertical 采样一条边，返回五像素容差内最高连续支持率
fn side(edges: &Gray, fixed: i32, mut start: i32, mut end: i32, vertical: bool) -> f32 {
    let limit = if vertical { edges.width } else { edges.height } as i32;
    if fixed <= 1 || fixed + 2 >= limit {
        return 0.85;
    }
    let margin = ((end - start) / 10).min(12);
    start += margin;
    end -= margin;
    if end <= start {
        return 0.0;
    }
    let step = ((end - start) / 160).max(1) as usize;
    let mut best = 0.0f32;
    for across in (fixed - 2).max(0)..=(fixed + 2).min(limit - 1) {
        let mut supported = 0;
        let mut samples = 0;
        for along in (start..end).step_by(step) {
            let hit = if vertical {
                edges.at(across as usize, along as usize) != 0
            } else {
                edges.at(along as usize, across as usize) != 0
            };
            supported += i32::from(hit);
            samples += 1;
        }
        best = best.max(supported as f32 / samples.max(1) as f32);
    }
    best
}

/// 【区域识别】【四边支持】按 field、rect 读取左、上、右、下方向支持率，返回四项评分
pub(super) fn sides(field: &Field, rect: PixelRect) -> [f32; 4] {
    [
        side(&field.vertical, rect.left, rect.top, rect.bottom, true),
        side(&field.horizontal, rect.top, rect.left, rect.right, false),
        side(&field.vertical, rect.right - 1, rect.top, rect.bottom, true),
        side(
            &field.horizontal,
            rect.bottom - 1,
            rect.left,
            rect.right,
            false,
        ),
    ]
}

/// 【区域识别】【内部密度】按 edges、rect 在内缩三像素后采样，返回内部边缘占比
pub(super) fn density(edges: &Gray, rect: PixelRect) -> f32 {
    if rect.width() <= 6 || rect.height() <= 6 {
        return 0.0;
    }
    let (mut count, mut samples) = (0, 0);
    for y in (rect.top + 3..rect.bottom - 3).step_by((rect.height() / 32).max(1) as usize) {
        for x in (rect.left + 3..rect.right - 3).step_by((rect.width() / 32).max(1) as usize) {
            count += i32::from(edges.at(x as usize, y as usize) != 0);
            samples += 1;
        }
    }
    count as f32 / samples.max(1) as f32
}

/// 【区域识别】【内缩支持】按 field、rect 读取内缩四像素后的平均边界支持率，返回纹理判断值
pub(super) fn inset(field: &Field, rect: PixelRect) -> f32 {
    if rect.width() < 12 || rect.height() < 12 {
        return 0.0;
    }
    let values = sides(field, rect.expanded(-4));
    (values[0] + values[1] + values[2] + values[3]) / 4.0
}
