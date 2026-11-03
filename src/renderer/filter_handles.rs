//! 【滤镜】【编辑控制框】绘制旋转外框和操作点，不写入持久滤镜或导出图像

use tiny_skia::{Color, PathBuilder, Stroke, StrokeDash};

use super::Painter;
use crate::features::filter::{RegionFilter, drag};

/// 【滤镜】【控制框绘制】把 filter 的八个缩放点和旋转把手绘入 painter；使用统一颜色和紧凑尺寸，无返回值
pub(crate) fn draw(painter: &mut Painter<'_>, filter: RegionFilter) {
    let color = crate::config::get().annotations.handles.color.get().color();
    let shadow = Color::from_rgba8(8, 13, 19, 230);
    let unit = filter.style_scale;
    if let Some(path) = drag::frame(filter).and_then(|rect| filter.path(rect)) {
        painter.stroke_path(
            &path,
            shadow,
            &Stroke {
                width: 3.0 * unit,
                ..Stroke::default()
            },
        );
        painter.stroke_path(
            &path,
            color,
            &Stroke {
                width: 1.2 * unit,
                dash: StrokeDash::new(vec![4.0 * unit, 3.0 * unit], 0.0),
                ..Stroke::default()
            },
        );
    }
    let mut points = drag::controls(filter)
        .into_iter()
        .map(|(_, point)| point)
        .collect::<Vec<_>>();
    if let Some(handle) = drag::rotation_handle(filter)
        && let Some(rect) = drag::frame(filter)
    {
        let start = filter.to_visual(((rect.left() + rect.right()) / 2.0, rect.top()));
        let mut path = PathBuilder::new();
        path.move_to(start.0, start.1);
        path.line_to(handle.0, handle.1);
        if let Some(path) = path.finish() {
            painter.stroke_path(
                &path,
                shadow,
                &Stroke {
                    width: 3.0 * unit,
                    ..Stroke::default()
                },
            );
            painter.stroke_path(
                &path,
                color,
                &Stroke {
                    width: unit,
                    ..Stroke::default()
                },
            );
        }
        points.push(handle);
    }
    for point in points {
        painter.fill_circle(point.0, point.1, 4.0 * unit, shadow);
        painter.fill_circle(point.0, point.1, 2.5 * unit, color);
    }
}
