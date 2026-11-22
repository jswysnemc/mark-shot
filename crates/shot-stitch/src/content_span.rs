//! 【长图拼接】【正文边界】细化混合采样带，固定侧栏不参与正文接缝核验

use crate::long_image::LongImage;
use shot_image::Image;

/// 【长图拼接】【横向复核】按 long、previous、frame、position、span、excluded 返回清晰运动正文范围，无法安全缩窄时返回包含全部运动列的范围供重采样校验
pub(crate) fn refine(
    long: &LongImage,
    previous: Option<&Image>,
    frame: &Image,
    position: i32,
    span: (i32, i32),
    excluded: (i32, i32),
) -> (i32, i32) {
    let start = excluded.0.max(position + excluded.0).max(0);
    let end = (long.height() - excluded.1).min(position + frame.height() - excluded.1);
    if end <= start {
        return span;
    }
    // 1. 【长图拼接】【分散原始行】复核成本仅与视口宽度有关，不扫描完整长图
    let count = (end - start).min(32);
    let rows: Vec<_> = (0..count)
        .map(|index| {
            let y = start + (end - start) * index / count;
            (long.row(y), frame.row(y - position), y - position)
        })
        .collect();
    let mut left = span.0.max(0);
    let mut right = (span.0 + span.1).min(frame.width());
    let columns: Vec<_> = (0..frame.width())
        .map(|x| {
            let motion = previous.map_or(0, |last| {
                rows.iter()
                    .filter(|(_, new, y)| {
                        if *y >= last.height() {
                            return false;
                        }
                        let old = last.row(*y)[x as usize];
                        [0, 8, 16].into_iter().any(|shift| {
                            ((old >> shift) & 255).abs_diff((new[x as usize] >> shift) & 255) > 1
                        })
                    })
                    .count()
            });
            // 2. 【长图拼接】【签名碰撞复核】统计不变的像素仍可能变化，整列运动不能被采样带掩盖
            if motion * 4 > rows.len() {
                left = left.min(x);
                right = right.max(x + 1);
            }
            motion
        })
        .collect();
    let mut run = left;
    let mut moving = false;
    let mut best = left..left;
    for x in left..right {
        let clean = rows
            .iter()
            .filter(|(old, new, _)| {
                let a = old[x as usize];
                let b = new[x as usize];
                [0, 8, 16]
                    .into_iter()
                    .all(|shift| ((a >> shift) & 255).abs_diff((b >> shift) & 255) <= 1)
            })
            .count();
        // 3. 【长图拼接】【侧栏排除】允许少量局部遮挡；固定侧栏在错开的文档行上不能成为正文边界
        if clean * 4 >= rows.len() * 3 {
            moving |= columns[x as usize] > 0;
            if moving && x + 1 - run > best.end - best.start {
                best = run..x + 1;
            }
        } else {
            run = x + 1;
            moving = false;
        }
    }
    // 4. 【长图拼接】【正文完整性】只有静止侧栏或空白可以排除，另一段运动正文不能因不匹配而被隐藏
    let hides_content = columns
        .iter()
        .enumerate()
        .any(|(column, &motion)| motion * 4 > rows.len() && !best.contains(&(column as i32)));
    if best.end - best.start >= 8 && !hides_content {
        (best.start, best.end - best.start)
    } else {
        (left, right - left)
    }
}
