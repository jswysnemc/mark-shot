//! 【滤镜】【编辑控制框】共享原版虚线、方块控制点与旋转把手

use super::Painter;
use crate::{
    features::annotation_selection::{chrome, region_frame},
    types::{Annotation, AnnotationShape},
    ui::annotation_selection::draw,
};

/// 【滤镜】【控制框绘制】按 annotation 将带留白的选框和把手绘入 painter，不修改持久图像
pub(crate) fn draw(painter: &mut Painter<'_>, annotation: &Annotation) {
    let AnnotationShape::Redact { filter } = annotation.shape else {
        return;
    };
    let window = painter.window_units();
    let Some(rect) = region_frame::bounds(annotation, window) else {
        return;
    };
    if let Some(path) = filter.path(rect) {
        draw::outline(painter, &path, 1.0);
    }
    for point in chrome::handles(chrome::corners(rect)) {
        draw::square(painter, filter.to_visual(point), 1.0);
    }
    if let Some((start, handle)) = region_frame::connector(annotation, window) {
        draw::rotation_points(painter, start, handle, 1.0);
    }
}
