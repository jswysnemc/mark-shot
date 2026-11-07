//! 【放大镜】【编辑控制点】双框共用原版虚线与方块控制点，不进入导出

use super::Painter;
use crate::{
    features::magnifier::{drag, draw as geometry},
    types::{Annotation, AnnotationShape},
    ui::annotation_selection::draw,
};

/// 【放大镜】【双框绘制】按 annotation 绘制两个区域和原生旋转控制点，无返回值，不修改采样图像
pub(crate) fn draw(painter: &mut Painter<'_>, annotation: &Annotation) {
    let AnnotationShape::Magnifier { magnifier } = annotation.shape else {
        return;
    };
    let unit = 1.0;
    for rect in [magnifier.lens_rect(), magnifier.source_rect()]
        .into_iter()
        .flatten()
    {
        if let Some(path) = geometry::frame_path(magnifier, rect) {
            draw::outline(painter, &path, unit);
        }
    }
    for (_, point) in drag::controls(magnifier) {
        draw::square(painter, point, unit);
    }
    if let Some((start, handle)) =
        crate::features::annotation_selection::rotation::connector(annotation)
    {
        draw::rotation(painter, start, handle, unit);
    }
}
