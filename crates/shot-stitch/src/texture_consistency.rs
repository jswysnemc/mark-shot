//! 【长图拼接】【纹理复核】分散匹配必须获得主要正文纹理支持，空白和重复头像不能掩盖标题替换

use crate::long_image::LongImage;
use shot_image::Image;
use std::ops::Range;

/// 【长图拼接】【颜色距离】按两个 ARGB 像素返回 RGB 通道最大距离，不比较透明度
fn distance(first: u32, second: u32) -> u32 {
    (first & 255)
        .abs_diff(second & 255)
        .max(((first >> 8) & 255).abs_diff((second >> 8) & 255))
        .max(((first >> 16) & 255).abs_diff((second >> 16) & 255))
}

/// 【长图拼接】【纹理支持】按 long、frame、position、span 和 range 核验对齐纹理，至少四分之三纹理一致时返回 true，不分配像素副本
pub(crate) fn matches(
    long: &LongImage,
    frame: &Image,
    position: i32,
    span: (i32, i32),
    range: Range<i32>,
) -> bool {
    let left = span.0.max(0) as usize;
    let right = (span.0 + span.1).min(frame.width()).max(0) as usize;
    let mut supported = 0_u64;
    let mut total = 0_u64;
    // 1. 【长图拼接】【完整纹理】检查水平和垂直邻接边缘，避免文字像素反转仍保留亮度统计而误拼
    for y in range.clone() {
        let old = long.row(y);
        let new = frame.row(y - position);
        let above = (y > range.start).then(|| (long.row(y - 1), frame.row(y - position - 1)));
        for x in left..right {
            let mut weight = 0;
            let mut clean = distance(old[x], new[x]) <= 1;
            if x > left {
                weight = distance(old[x], old[x - 1]).max(distance(new[x], new[x - 1]));
                clean &= distance(old[x - 1], new[x - 1]) <= 1;
            }
            if let Some((old_above, new_above)) = above {
                weight = weight
                    .max(distance(old[x], old_above[x]))
                    .max(distance(new[x], new_above[x]));
                clean &= distance(old_above[x], new_above[x]) <= 1;
            }
            // 2. 【长图拼接】【空白排除】平坦背景权重为零，局部头像只能贡献自身纹理，不能替代整块正文
            total += u64::from(weight);
            if clean {
                supported += u64::from(weight);
            }
        }
    }
    total > 0 && supported * 4 >= total * 3
}
