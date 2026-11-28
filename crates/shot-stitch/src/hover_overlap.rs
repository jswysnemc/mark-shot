//! 【滚动截图】【悬停底色】只比较当前重叠区，识别列表底色替换与文字抗锯齿，不修改原始像素

use crate::long_image::LongImage;
use shot_image::Image;
use std::ops::Range;

/// 【滚动截图】【通道差值】按前后两个 RGB 像素返回三个有符号通道差值
fn delta(old: u32, new: u32) -> [i32; 3] {
    [0, 8, 16].map(|shift| ((new >> shift) & 255) as i32 - ((old >> shift) & 255) as i32)
}

/// 【滚动截图】【底色解释】按前后两行判断变化是否仅来自同一底色替换；不能用底色解释文字移动或内容替换
fn background_change(old: &[u32], new: &[u32]) -> bool {
    // 1. 【滚动截图】【底色采样】只从两侧都是平坦颜色的连续像素取样，孤立的文字边缘不能定义底色
    let Some(index) = (2..old.len()).find(|&index| {
        old[index] != new[index]
            && old[index - 2..index]
                .iter()
                .all(|&pixel| pixel == old[index])
            && new[index - 2..index]
                .iter()
                .all(|&pixel| pixel == new[index])
    }) else {
        return false;
    };
    let pair = (old[index], new[index]);
    let base = delta(pair.0, pair.1);
    let axis = (0..3).max_by_key(|&channel| base[channel].abs()).unwrap();
    let scale = base[axis];
    if scale == 0 {
        return false;
    }
    // 2. 【滚动截图】【字形保护】同一前景叠在不同底色上，各通道变化应是底色差乘以相同覆盖比例
    old.iter().zip(new).all(|(&before, &after)| {
        if before == after || (before, after) == pair {
            return true;
        }
        let change = delta(before, after);
        let fraction = change[axis] * scale.signum();
        (0..=scale.abs() + 1).contains(&fraction)
            && (0..3).all(|channel| {
                (change[channel] * scale - base[channel] * change[axis]).abs() <= 2 * scale.abs()
            })
    })
}

/// 【滚动截图】【悬停复核】按长图、当前帧、对齐位置、正文宽度和重叠范围逐行核验，返回是否只有局部底色发生变化
pub(super) fn matches(
    long: &LongImage,
    frame: &Image,
    position: i32,
    span: (i32, i32),
    range: Range<i32>,
) -> bool {
    let columns = span.0 as usize..(span.0 + span.1) as usize;
    let mut changed = false;
    for vertical in range {
        let old = &long.row(vertical)[columns.clone()];
        let new = &frame.row(vertical - position)[columns.clone()];
        if old != new {
            if !background_change(old, new) {
                return false;
            }
            changed = true;
        }
    }
    changed
}
