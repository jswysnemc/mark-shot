//! 【标注样式】【编辑入口】工具创建与属性控件共用原图换算，导航只影响绘制投影

use super::EditorState;
use crate::{
    features::annotation_style::units::Units,
    types::{Annotation, AnnotationShape},
};
use tiny_skia::Color;

impl EditorState {
    /// 【标注样式】【当前单位】无额外参数，返回冻结原图相对稳定画布的换算规则，无有效空间时使用一比一单位
    pub(crate) fn annotation_units(&self) -> Units {
        let pixel_scale = crate::tools::drawing_space::DrawingSpace::from_editor(self)
            .map_or(1.0, |space| space.pixel_scale);
        if let Some(file) = &self.file_canvas {
            Units::for_image(f64::from(file.original.width()) / f64::from(file.viewport.width()))
        } else {
            Units::for_capture(f64::from(pixel_scale))
        }
    }

    /// 【标注样式】【创建换算】按annotation把新工具的原图笔宽与固定尺寸转换到画布，无返回值；文字已有独立原图字号，不重复转换
    pub(crate) fn adopt_annotation_style(&self, annotation: &mut Annotation) {
        let units = self.annotation_units();
        let scale = units.canvas_per_image();
        annotation.style_scale = scale;
        // 1. 【标注样式】【原图边界】保存冻结画布边界，后续控制框和旋转轴心使用同一原图范围
        if annotation.frozen_bounds.is_none() {
            annotation.frozen_bounds = crate::tools::drawing_space::DrawingSpace::from_editor(self)
                .map(|space| space.bounds);
        }
        // 2. 【标注样式】【尺寸换算】只调整样式，不缩放当前指针几何；滤镜强度继续使用原图像素
        match &mut annotation.shape {
            AnnotationShape::Text { .. } => return,
            AnnotationShape::Number { mark } => mark.style_scale = scale,
            AnnotationShape::Magnifier { magnifier } => magnifier.style_scale = scale,
            AnnotationShape::Ruler { measurement } => measurement.style_scale = units.ruler_scale(),
            AnnotationShape::Redact { filter } => filter.style_scale = scale,
            AnnotationShape::Arrow { line, .. } | AnnotationShape::Line { line, .. } => {
                line.style_scale = scale
            }
            _ => {}
        }
        annotation.stroke_width = units.canvas(annotation.stroke_width);
        // 3. 【标注样式】【柔和正文】箭头和矩形保留轻微投影，其余标注继续使用原版无阴影正文
        annotation.shadow_color = shadow_for_shape(&annotation.shape, annotation.color);
    }
}

/// 【标注样式】【投影形状】按 shape 与 color 返回创建和改色共用的阴影颜色，未列出的形状保持透明
pub(crate) fn shadow_for_shape(shape: &AnnotationShape, color: Color) -> Color {
    match shape {
        AnnotationShape::Arrow { .. } | AnnotationShape::Rectangle { .. } => {
            let mut shadow = crate::renderer::shadow_color_for(color);
            let alpha = crate::config::get().annotations.shadow.shape_alpha;
            shadow.set_alpha(
                shadow.alpha()
                    * if alpha.is_finite() {
                        alpha.clamp(0.0, 1.0)
                    } else {
                        0.55
                    },
            );
            shadow
        }
        AnnotationShape::Text { .. } => crate::renderer::shadow_color_for(color),
        _ => Color::TRANSPARENT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::line::LineSkeleton;

    /// 【标注样式测试】【投影强度】箭头和矩形使用淡化的共享阴影，其他正文保持透明
    #[test]
    fn arrows_and_rectangles_receive_a_lighter_shadow() {
        let color = Color::from_rgba8(255, 0, 0, 255);
        let full = crate::renderer::shadow_color_for(color);
        let rectangle = AnnotationShape::Rectangle {
            radius: 0.0,
            fixed_ratio: false,
            start: (0.0, 0.0),
            end: (40.0, 20.0),
            filled: false,
        };
        let arrow = AnnotationShape::Arrow {
            line: LineSkeleton::new((0.0, 0.0), (40.0, 20.0)).unwrap(),
            style: Default::default(),
            line_style: Default::default(),
        };
        for shape in [rectangle, arrow] {
            let shadow = shadow_for_shape(&shape, color);
            assert!(shadow.alpha() > 0.0);
            assert!(shadow.alpha() < full.alpha());
        }
        let line = AnnotationShape::Line {
            line: LineSkeleton::new((0.0, 0.0), (40.0, 20.0)).unwrap(),
            style: Default::default(),
        };
        assert_eq!(shadow_for_shape(&line, color), Color::TRANSPARENT);
    }
}
