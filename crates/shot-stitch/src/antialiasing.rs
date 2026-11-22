//! 【长图拼接】【抗锯齿校验】核验灰度与子像素渲染切换，空白不提供证据，原始像素不做归一化

use shot_image::Image;

/// 【长图拼接】【通道范围】按 ARGB pixel 返回 RGB 最小值与最大值，不比较透明度
fn channels(pixel: u32) -> (u32, u32) {
    let red = pixel >> 16 & 255;
    let green = pixel >> 8 & 255;
    let blue = pixel & 255;
    (red.min(green).min(blue), red.max(green).max(blue))
}

/// 【长图拼接】【颜色距离】按两个 ARGB 像素返回 RGB 通道最大距离，不比较透明度
fn distance(first: u32, second: u32) -> u32 {
    (first & 255)
        .abs_diff(second & 255)
        .max(((first >> 8) & 255).abs_diff((second >> 8) & 255))
        .max(((first >> 16) & 255).abs_diff((second >> 16) & 255))
}

/// 【长图拼接】【亮度覆盖】按中心及两侧像素返回一比二比一加权 RGB 总量，比较阶段避免取整误差，不修改图像
fn coverage(center: u32, before: u32, after: u32) -> u32 {
    let sum = |pixel: u32| (pixel >> 16 & 255) + (pixel >> 8 & 255) + (pixel & 255);
    sum(before) + 2 * sum(center) + sum(after)
}

/// 【长图拼接】【边缘方向】按两帧中心与邻接像素返回亮度边缘是否同向，只有不足四级平均亮度的弱边缘允许方向改变
fn same_direction(old: u32, old_neighbor: u32, new: u32, new_neighbor: u32) -> bool {
    let sum = |pixel: u32| ((pixel >> 16 & 255) + (pixel >> 8 & 255) + (pixel & 255)) as i32;
    let old_edge = sum(old) - sum(old_neighbor);
    let new_edge = sum(new) - sum(new_neighbor);
    old_edge.signum() == new_edge.signum() || old_edge.abs().max(new_edge.abs()) <= 12
}

/// 【长图拼接】【覆盖率校验】按 old、new 返回像素是否一致或符合灰度与子像素切换；灰度值必须落在另一像素的通道覆盖范围内
fn compatible(old: u32, new: u32) -> bool {
    if distance(old, new) <= 1 {
        return true;
    }
    let (old_min, old_max) = channels(old);
    let (new_min, new_max) = channels(new);
    // 1. 【长图拼接】【渲染约束】只接收一侧灰度、一侧彩色的覆盖率变化，两个彩色像素不能借此容忍不同颜色
    if old_max - old_min <= 1 && new_max - new_min > 1 {
        old_min.saturating_add(2) >= new_min && old_max <= new_max + 2
    } else if new_max - new_min <= 1 && old_max - old_min > 1 {
        new_min.saturating_add(2) >= old_min && new_max <= old_max + 2
    } else {
        false
    }
}

/// 【长图拼接】【渲染重叠】比较 previous 与 current，offset 表示 current 相对 previous 的文档位置，正值向后移动；span 是横向左坐标和宽度，excluded 是上下排除行数，返回可信重叠行数，不分配像素缓存
pub(crate) fn overlap(
    previous: &Image,
    current: &Image,
    offset: i32,
    span: (i32, i32),
    excluded: (i32, i32),
) -> i32 {
    // 1. 【长图拼接】【重叠范围】按真实位移计算共同文档行，排除固定边缘并限制横向扫描范围
    let start = excluded.0.max(offset + excluded.0).max(0);
    let end = (previous.height() - excluded.1).min(offset + current.height() - excluded.1);
    let left = span.0.max(0) as usize;
    let right = (span.0 + span.1)
        .max(0)
        .min(previous.width())
        .min(current.width()) as usize;
    let (mut total, mut supported, mut changed, mut explained) = (0_u64, 0_u64, 0_u64, 0_u64);
    let mut geometry = [0_u64; 2];
    let (mut direction_total, mut direction_supported) = (0_u64, 0_u64);
    let mut textured_rows = 0;
    for y in start..end {
        let old = previous.row(y);
        let new = current.row(y - offset);
        let above = (y > start).then(|| (previous.row(y - 1), current.row(y - offset - 1)));
        let below = (y + 1 < end).then(|| (previous.row(y + 1), current.row(y - offset + 1)));
        let mut row_weight = 0;
        let mut left_compatible = true;
        for x in left..right {
            let same = distance(old[x], new[x]) <= 1;
            let pixel_compatible = compatible(old[x], new[x]);
            let mut clean = pixel_compatible && left_compatible;
            let mut weight = 0;
            // 2. 【长图拼接】【邻接字形】当前像素与水平、垂直邻居都需一致，空白和少量相同图标不能替代正文
            if x > left {
                weight = distance(old[x], old[x - 1]).max(distance(new[x], new[x - 1]));
                direction_total += u64::from(weight);
                if same_direction(old[x], old[x - 1], new[x], new[x - 1]) {
                    direction_supported += u64::from(weight);
                }
            }
            if let Some((old_above, new_above)) = above {
                let vertical_weight =
                    distance(old[x], old_above[x]).max(distance(new[x], new_above[x]));
                direction_total += u64::from(vertical_weight);
                if same_direction(old[x], old_above[x], new[x], new_above[x]) {
                    direction_supported += u64::from(vertical_weight);
                }
                weight = weight
                    .max(distance(old[x], old_above[x]))
                    .max(distance(new[x], new_above[x]));
                clean &= compatible(old_above[x], new_above[x]);
            }
            let weight = u64::from(weight);
            row_weight += weight;
            total += weight;
            if clean {
                supported += weight;
            }
            if !same {
                changed += weight;
                if pixel_compatible {
                    explained += weight;
                }
            }
            if weight > 0 {
                // 3. 【长图拼接】【字形覆盖】核验同一方向的局部亮度，防止任意宽通道范围掩盖正文替换；双轴都保留另一方向分辨率
                let before = x.saturating_sub(1).max(left);
                let after = (x + 1).min(right - 1);
                let horizontal = coverage(old[x], old[before], old[after]).abs_diff(coverage(
                    new[x],
                    new[before],
                    new[after],
                ));
                let (old_above, new_above) = above.unwrap_or((old, new));
                let (old_below, new_below) = below.unwrap_or((old, new));
                let vertical = coverage(old[x], old_above[x], old_below[x]).abs_diff(coverage(
                    new[x],
                    new_above[x],
                    new_below[x],
                ));
                geometry[0] += if horizontal <= 12 * 12 { weight } else { 0 };
                geometry[1] += if vertical <= 12 * 12 { weight } else { 0 };
            }
            left_compatible = pixel_compatible;
        }
        textured_rows += i32::from(row_weight > 0);
    }
    // 4. 【长图拼接】【完整证据】至少八行纹理，五分之四纹理及变化符合渲染切换，同一方向至少十九分之二十纹理保持字形覆盖
    if textured_rows >= 8
        && changed > 0
        && supported * 5 >= total * 4
        && explained * 5 >= changed * 4
        && geometry[0].max(geometry[1]) * 20 >= total * 19
        && direction_supported * 20 >= direction_total * 19
    {
        (end - start).max(0)
    } else {
        0
    }
}
