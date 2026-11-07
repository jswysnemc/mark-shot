//! 【标注样式】【编辑入口】工具创建与属性控件共用原图换算，导航只影响绘制投影

use super::EditorState;
use crate::{
    features::annotation_style::units::Units,
    types::{Annotation, AnnotationShape},
};

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
        // 3. 【标注样式】【标准正文】原版普通标注直接绘制正文，工具创建不附加额外的阴影或画笔光晕
        annotation.shadow_color = tiny_skia::Color::TRANSPARENT;
    }
}
