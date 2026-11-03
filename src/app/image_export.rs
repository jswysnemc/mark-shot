use std::collections::HashMap;

use crate::editor::EditorState;

/// 【图片编辑】【原图导出】在原始像素上合成标注并保留透明度；返回 PNG、逻辑位置与像素比例
pub(super) fn render(editor: &mut EditorState) -> Option<(Vec<u8>, (i32, i32), f32)> {
    let file = editor.file_canvas.as_ref()?;
    let crop = file.crop(editor.selection.zone.unwrap_or(file.viewport))?;
    let mut pixels = crop.pixels;
    let origin = (
        crop.global_origin.0 * crop.scale,
        crop.global_origin.1 * crop.scale,
    );
    let mut editors = HashMap::new();
    let blurs = HashMap::new();
    let bounds = tiny_skia::Rect::from_xywh(
        origin.0,
        origin.1,
        pixels.width() as f32,
        pixels.height() as f32,
    )?;
    for annotation in &editor.annotations {
        let annotation = annotation.scaled(crop.scale);
        crate::renderer::draw_annotation(
            &mut crate::renderer::Painter::new(&mut pixels)
                .with_frozen_image(
                    crate::renderer::FrozenImage::image(&file.original, file.viewport)
                        .mapped(crop.scale, (0.0, 0.0)),
                )
                .with_drawing_bounds(Some(bounds))
                .with_text(&mut editor.text.font_system, &mut editor.text.swash_cache)
                .with_origin(origin),
            &annotation,
            false,
            &mut editors,
            &blurs,
            None,
        );
    }
    let rulers = editor.annotations.iter().filter_map(|annotation| {
        if let crate::types::AnnotationShape::Ruler { measurement } = annotation.shape {
            Some((
                measurement.scaled(crop.scale),
                annotation.stroke_width * crop.scale,
            ))
        } else {
            None
        }
    });
    let labels =
        crate::features::ruler::labels::prepare_all(rulers, bounds, &mut editor.text.font_system);
    crate::renderer::ruler_overlay::draw_labels(
        &mut crate::renderer::Painter::new(&mut pixels)
            .with_origin(origin)
            .with_text(&mut editor.text.font_system, &mut editor.text.swash_cache),
        &labels,
    );
    Some((
        crate::utils::encode_png_with_alpha(pixels),
        (
            crop.global_origin.0.round() as i32,
            crop.global_origin.1.round() as i32,
        ),
        crop.scale,
    ))
}
