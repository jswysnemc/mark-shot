//! 【取色标注】【动态采样】指向点移动后从冻结原图刷新颜色，标签与属性共用结果

use crate::{
    renderer::FrozenImage,
    types::{Annotation, AnnotationShape},
};
use tiny_skia::Color;

/// 【取色标注】【点位颜色】按 image 与 point 返回反预乘颜色，原图范围外返回空值
pub(crate) fn at(image: FrozenImage<'_>, point: (f32, f32)) -> Option<Color> {
    let pixel = image.sample_nearest(point)?;
    let pixel = tiny_skia::PremultipliedColorU8::from_rgba(pixel[0], pixel[1], pixel[2], pixel[3])?
        .demultiply();
    Some(Color::from_rgba8(
        pixel.red(),
        pixel.green(),
        pixel.blue(),
        pixel.alpha(),
    ))
}

/// 【取色标注】【几何刷新】按 image 更新 annotation 指向点颜色，返回变化后的颜色；其他标注不处理
pub(crate) fn refresh(annotation: &mut Annotation, image: FrozenImage<'_>) -> Option<Color> {
    let AnnotationShape::ColorSample { sample } = &mut annotation.shape else {
        return None;
    };
    let color = at(image, sample.source)?;
    sample.color = color;
    annotation.color = color;
    Some(color)
}
