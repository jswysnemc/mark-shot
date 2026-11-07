//! 【滤镜】【编辑控制框】共享原版虚线、方块控制点与旋转把手

use super::Painter;
use crate::{
    features::filter::{RegionFilter, drag},
    ui::annotation_selection::draw,
};

/// 【滤镜】【控制框绘制】按 filter 把旋转外框与控制点绘入 painter，无返回值，不进入持久图像
pub(crate) fn draw(painter: &mut Painter<'_>, filter: RegionFilter) {
    let unit = 1.0;
    if let Some(path) = drag::frame(filter).and_then(|rect| filter.path(rect)) {
        draw::outline(painter, &path, unit);
    }
    for (_, point) in drag::controls(filter) {
        draw::square(painter, point, unit);
    }
    if let Some(handle) = drag::rotation_handle(filter)
        && let Some(rect) = drag::frame(filter)
    {
        let start = filter.to_visual(((rect.left() + rect.right()) * 0.5, rect.top()));
        let handle = crate::features::annotation_selection::rotation::projected(
            start,
            handle,
            1.0 / filter.style_scale,
        );
        draw::rotation(painter, start, handle, unit);
    }
}
