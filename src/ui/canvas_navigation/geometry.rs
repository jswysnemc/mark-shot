//! 【画布滚动条】【控件几何】共享绘制和命中使用的Qt页长、最小滑块及边距

use crate::features::canvas_navigation::{
    coordinates::{Point, Rect},
    scrollbars::Axis,
};

/// 【画布滚动条】【滑块几何】按axis、horizontal和hover返回轨道与滑块，空范围返回空值
pub(crate) fn parts(axis: Axis, horizontal: bool, hover: bool) -> Option<(Rect, Rect)> {
    if !axis.visible || axis.maximum <= 0 {
        return None;
    }
    let mut track = axis.rect;
    let (along, cross) = if hover { (2.0, 2.0) } else { (3.0, 4.0) };
    if horizontal {
        track.x += along;
        track.y += cross;
        track.width -= 2.0 * along;
        track.height -= 2.0 * cross;
    } else {
        track.x += cross;
        track.y += along;
        track.width -= 2.0 * cross;
        track.height -= 2.0 * along;
    }
    let length = if horizontal {
        track.width
    } else {
        track.height
    };
    if length <= 0.0 {
        return None;
    }
    let thumb_length = (length * f64::from(axis.page_step)
        / (f64::from(axis.maximum) + f64::from(axis.page_step)))
    .max(36.0)
    .min(length);
    let position = (length - thumb_length) * f64::from(axis.value) / f64::from(axis.maximum);
    let mut thumb = track;
    if horizontal {
        thumb.x += position;
        thumb.width = thumb_length;
    } else {
        thumb.y += position;
        thumb.height = thumb_length;
    }
    Some((track, thumb))
}

/// 【画布滚动条】【轴命中】按axis与point返回是否命中可见滚动条的完整控件范围
pub(crate) fn hit(axis: Axis, point: Point) -> bool {
    axis.visible && axis.rect.contains(point)
}
