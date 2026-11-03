//! 【渲染】【平滑路径】共用画笔、荧光笔与激光的二次曲线路径计算

use tiny_skia::{Path, PathBuilder, Rect};

/// 【渲染】【平滑路径】将 points 转为平滑路径；skip 跳过已缓存段，clip 筛选逻辑可见区域，返回可绘制路径
pub fn freehand_path(points: &[(f32, f32)], skip: usize, clip: Option<Rect>) -> Option<Path> {
    let mid = |a: (f32, f32), b: (f32, f32)| ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    let mut builder = PathBuilder::new();
    match points {
        [] => return None,
        [point] => {
            builder.move_to(point.0, point.1);
            builder.line_to(point.0, point.1);
        }
        [start, end] => {
            builder.move_to(start.0, start.1);
            builder.line_to(end.0, end.1);
        }
        _ => {
            let mut open = false;
            for index in 1 + skip..points.len() {
                let start = if index == 1 {
                    points[0]
                } else {
                    mid(points[index - 1], points[index])
                };
                let (control, end) = if index < points.len() - 1 {
                    (Some(points[index]), mid(points[index], points[index + 1]))
                } else {
                    (None, points[index])
                };

                // 1. 【渲染】【平滑路径】用控制点包围区域筛选路径段，保留曲线边缘
                let visible = clip.is_none_or(|area| {
                    let control = control.unwrap_or(start);
                    start.0.min(end.0).min(control.0) <= area.right()
                        && start.0.max(end.0).max(control.0) >= area.left()
                        && start.1.min(end.1).min(control.1) <= area.bottom()
                        && start.1.max(end.1).max(control.1) >= area.top()
                });
                if !visible {
                    open = false;
                    continue;
                }
                if !open {
                    builder.move_to(start.0, start.1);
                    open = true;
                }
                match control {
                    Some(point) => builder.quad_to(point.0, point.1, end.0, end.1),
                    None => builder.line_to(end.0, end.1),
                }
            }
        }
    }
    builder.finish()
}
