//! 【普通标注】【正文调度】矢量正文直接旋转路径，文字按可见区域生成图像，不改变标注数据

use super::Painter;
use crate::{
    features::annotation_rotation::geometry,
    types::{Annotation, AnnotationShape},
};

/// 【普通标注】【旋转绘制】按 painter、annotation 和 editing 执行 body，无返回值，原版内联文字编辑器保持未旋转
pub(super) fn draw(
    painter: &mut Painter<'_>,
    annotation: &Annotation,
    painted: Option<tiny_skia::Rect>,
    editing: bool,
    mut body: impl FnMut(&mut Painter<'_>),
) {
    if !annotation.uses_generic_rotation()
        || geometry::normalized(annotation.rotation_degrees) == 0.0
        || editing && matches!(annotation.shape, AnnotationShape::Text { .. })
    {
        body(painter);
    } else {
        let mut rendered = annotation.clone();
        if let AnnotationShape::Text { frame, .. } = &mut rendered.shape {
            frame.painted = painted;
        }
        super::rotation_tiles::draw(painter, &rendered, |painter| {
            if matches!(annotation.shape, AnnotationShape::Text { .. }) {
                super::rotated_text::draw(painter, &rendered, &mut body);
            } else {
                painter.transformed(
                    geometry::transform(annotation.rotation_center(), annotation.rotation_degrees),
                    &mut body,
                );
            }
        });
    }
}
