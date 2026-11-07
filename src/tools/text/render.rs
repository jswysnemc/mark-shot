//! 【标注】【文字绘制】绘制正文、描边和编辑选区

use super::*;

/// 【文字标注】【完整绘制】将 ann 与 editor 绘入 pixmap；font_system、swash_cache 提供字形，offset、scale 映射坐标，is_editing 控制光标与选区，无返回值
pub fn render_text_annotation(
    ann: &Annotation,
    editor: &mut Editor<'static>,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut SwashCache,
    pixmap: &mut PixmapMut<'_>,
    offset: (f32, f32),
    scale: f32,
    is_editing: bool,
) {
    render_text_annotation_with_caret(
        ann,
        editor,
        font_system,
        swash_cache,
        pixmap,
        offset,
        scale,
        is_editing,
        true,
        None,
    );
}

/// 【文字标注】【光标分离】用 ann、editor、font_system 与 swash_cache 绘制到 pixmap；offset、scale、pixel_phase 保留原像素相位，is_editing 保留选区，caret_visible 控制光标，无返回值
pub fn render_text_annotation_with_caret(
    ann: &Annotation,
    editor: &mut Editor<'static>,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut SwashCache,
    pixmap: &mut PixmapMut<'_>,
    offset: (f32, f32),
    scale: f32,
    is_editing: bool,
    caret_visible: bool,
    pixel_phase: Option<(f64, f64)>,
) {
    let AnnotationShape::Text {
        start,
        font_size,
        outline,
        appearance,
        ..
    } = &ann.shape
    else {
        return;
    };
    // 1. 【文字标注】【局部原点】局部缓冲保留原画布相位，先恢复物理原点再减去，避免分数比例往返换算移动圆角覆盖
    let physical_origin = |coordinate: f32, phase: Option<f64>| {
        let raw = f64::from(coordinate) * f64::from(scale);
        phase.map_or(raw, |fraction| (raw - fraction).round() + fraction)
    };
    let origin = (
        physical_origin(offset.0, pixel_phase.map(|phase| phase.0)),
        physical_origin(offset.1, pixel_phase.map(|phase| phase.1)),
    );
    let bounds = text_bounds(ann, editor);
    if let Some(physical) = Rect::from_xywh(
        (f64::from(bounds.left()) * f64::from(scale) - origin.0) as f32,
        (f64::from(bounds.top()) * f64::from(scale) - origin.1) as f32,
        bounds.width() * scale,
        bounds.height() * scale,
    ) {
        super::background::draw(
            pixmap,
            physical,
            appearance.background,
            4.0 * appearance.size_scale * scale,
        );
    }
    let padding = crate::features::text::geometry::padding(appearance.size_scale);
    let base = ann.text_stored_bounds();
    let start = base.map_or(*start, |bounds| (bounds.left(), bounds.top()));
    let ann_x = (f64::from(start.0 + padding.0) * f64::from(scale) - origin.0) as f32;
    let ann_y = (f64::from(start.1 + padding.1) * f64::from(scale) - origin.1) as f32;
    let text_x = (ann_x + 0.5).floor();
    let text_y = (ann_y + 0.5).floor();

    let text_color = tiny_skia_to_cosmic(ann.color);
    let cursor_color = text_color;
    let sel_color = color::text_selection().cosmic();
    let sel_text_color = crate::config::get()
        .annotations
        .text
        .selected_text
        .get()
        .cosmic();
    let transparent = cosmic_text::Color::rgba(0, 0, 0, 0);

    let (cur_col, sel_col) = if is_editing {
        (
            if caret_visible {
                cursor_color
            } else {
                transparent
            },
            sel_color,
        )
    } else {
        (transparent, transparent)
    };

    editor.with_buffer_mut(|buf| {
        shape_entire_buffer(buf, font_system, false);
    });

    let outline_stroke_width = text_outline_stroke_width(*font_size) * scale;
    if let Some(outline_color) = outline
        && outline_stroke_width > 0.0
        // same snap as text uses
        && let Some(letters) =
            letters_path(editor, font_system, swash_cache, (text_x, text_y), scale)
    {
        draw_text_outline(
            pixmap,
            &letters,
            outline_stroke_width,
            *outline_color,
            ann.color,
        );
    }

    let p_width = pixmap.width() as i32;
    let p_height = pixmap.height() as i32;
    let pixels = pixmap.pixels_mut();

    crate::renderer::editor_text::draw_editor(
        editor,
        font_system,
        swash_cache,
        scale,
        [
            text_color,
            cur_col,
            sel_col,
            if is_editing {
                sel_text_color
            } else {
                text_color
            },
        ],
        |x, y, w, h, color| {
            let mut draw_w = w as f32;

            // 2. 【文字标注】【空行选择】只有零宽的换行选择需要补齐文字框，单像素字形不能按颜色误判为选择矩形
            if color == sel_col && w == 0 {
                draw_w = (ann.bbox.width() * scale - x as f32).max(min_width() * scale);
            }

            let start_x = text_x as i32 + x;
            let start_y = text_y as i32 + y;
            let w_int = draw_w.round() as i32;
            let h_int = h as i32;

            // 3. 【文字标注】【单次透明度】共享字形绘制已融合文字 alpha，此处只做预乘合成，光标与选区保留自身透明度
            let src_a = color.a() as u32;
            if src_a == 0 {
                return;
            }

            let src_r = (color.r() as u32 * src_a) / 255;
            let src_g = (color.g() as u32 * src_a) / 255;
            let src_b = (color.b() as u32 * src_a) / 255;
            let inv_a = 255 - src_a;

            for dy in 0..h_int {
                let py = start_y + dy;
                if py < 0 || py >= p_height {
                    continue;
                }

                for dx in 0..w_int {
                    let px = start_x + dx;
                    if px < 0 || px >= p_width {
                        continue;
                    }

                    let idx = (py * p_width + px) as usize;
                    let dst = pixels[idx];

                    let out_a = src_a + (dst.alpha() as u32 * inv_a) / 255;
                    let out_r = src_r + (dst.red() as u32 * inv_a) / 255;
                    let out_g = src_g + (dst.green() as u32 * inv_a) / 255;
                    let out_b = src_b + (dst.blue() as u32 * inv_a) / 255;

                    if let Some(c) = tiny_skia::PremultipliedColorU8::from_rgba(
                        out_r as u8,
                        out_g as u8,
                        out_b as u8,
                        out_a as u8,
                    ) {
                        pixels[idx] = c;
                    }
                }
            }
        },
    );
}

pub(super) fn letters_path(
    editor: &Editor<'static>,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut SwashCache,
    text_pos: (f32, f32),
    scale: f32,
) -> Option<Path> {
    let mut path_builder = PathBuilder::new();
    editor.with_buffer(|buf| {
        for run in buf.layout_runs() {
            for glyph in run.glyphs {
                let pixel_glyph = glyph.physical((0.0, run.line_y * scale), scale);
                let Some(glyph_shape) =
                    swash_cache.get_outline_commands(font_system, pixel_glyph.cache_key)
                else {
                    continue;
                };
                // cosmic-text works with whole pixels, and then adds whatever was left, we need to add that too
                let glyph_x =
                    text_pos.0 + pixel_glyph.x as f32 + pixel_glyph.cache_key.x_bin.as_float();
                let glyph_y = text_pos.1 + pixel_glyph.y as f32;
                for command in glyph_shape {
                    match *command {
                        Command::MoveTo(to) => path_builder.move_to(glyph_x + to.x, glyph_y - to.y),
                        Command::LineTo(to) => path_builder.line_to(glyph_x + to.x, glyph_y - to.y),
                        Command::QuadTo(ctrl, to) => path_builder.quad_to(
                            glyph_x + ctrl.x,
                            glyph_y - ctrl.y,
                            glyph_x + to.x,
                            glyph_y - to.y,
                        ),
                        Command::CurveTo(ctrl1, ctrl2, to) => path_builder.cubic_to(
                            glyph_x + ctrl1.x,
                            glyph_y - ctrl1.y,
                            glyph_x + ctrl2.x,
                            glyph_y - ctrl2.y,
                            glyph_x + to.x,
                            glyph_y - to.y,
                        ),
                        Command::Close => path_builder.close(),
                    }
                }
            }
        }
    });
    path_builder.finish()
}

// must be done before the text itself
pub(super) fn draw_text_outline(
    pixmap: &mut PixmapMut<'_>,
    letters: &Path,
    stroke_width: f32,
    mut color: tiny_skia::Color,
    text_color: tiny_skia::Color,
) {
    color.apply_opacity(text_color.alpha());
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    let stroke = Stroke {
        width: stroke_width,
        // to avoid sharp corners
        line_join: LineJoin::Round,
        ..Stroke::default()
    };

    if text_color.alpha() >= 1.0 {
        pixmap.stroke_path(letters, &paint, &stroke, Transform::identity(), None);
        return;
    }

    let pad = stroke_width / 2.0 + 1.0;
    let bounds = letters.bounds();
    let left = ((bounds.left() - pad).floor() as i32).max(0);
    let top = ((bounds.top() - pad).floor() as i32).max(0);
    let right = ((bounds.right() + pad).ceil() as i32).min(pixmap.width() as i32);
    let bottom = ((bounds.bottom() + pad).ceil() as i32).min(pixmap.height() as i32);
    if right <= left || bottom <= top {
        return;
    }
    let Some(mut layer) = Pixmap::new((right - left) as u32, (bottom - top) as u32) else {
        return;
    };
    let shift = Transform::from_translate(-left as f32, -top as f32);
    layer.stroke_path(letters, &paint, &stroke, shift, None);

    let mut eraser = Paint::default();
    eraser.anti_alias = true;
    eraser.blend_mode = BlendMode::DestinationOut;
    layer.fill_path(letters, &eraser, FillRule::Winding, shift, None);

    pixmap.draw_pixmap(
        left,
        top,
        layer.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}
