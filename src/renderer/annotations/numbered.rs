//! 【渲染】【编号箭头】绘制编号背景与数字

use super::*;

pub(super) fn contrasting_text_color(circle_color: Color) -> Color {
    let numbered = &crate::config::get().annotations.numbered;
    if luminance(circle_color) > numbered.digits_flip {
        numbered.dark_digits.color()
    } else {
        numbered.light_digits.color()
    }
}

pub(super) fn draw_numerated_arrow(
    p: &mut Painter,
    start: (f32, f32),
    end: (f32, f32),
    number: u32,
    color: Color,
    stroke_width: f32,
    shadow_color: Color,
) {
    let numbered = &crate::config::get().annotations.numbered;
    let (transform, shadow_transform) = transforms_for(p);

    let circle_radius = stroke_width * numbered.circle;

    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let len = (dx * dx + dy * dy).sqrt();

    let mut fill_paint = Paint::default();
    fill_paint.set_color(color);
    fill_paint.anti_alias = true;

    let mut shadow_paint = Paint::default();
    shadow_paint.set_color(shadow_color);
    shadow_paint.anti_alias = true;

    if len > 2.0 {
        let ux = dx / len;
        let uy = dy / len;

        let px = -uy;
        let py = ux;

        let base_half_width = circle_radius * numbered.tail_width;

        let base_left = (
            start.0 + px * base_half_width,
            start.1 + py * base_half_width,
        );
        let base_right = (
            start.0 - px * base_half_width,
            start.1 - py * base_half_width,
        );

        let mut pb = PathBuilder::new();
        pb.move_to(end.0, end.1);
        pb.line_to(base_left.0, base_left.1);
        pb.line_to(base_right.0, base_right.1);
        pb.close();

        if let Some(path) = pb.finish() {
            p.canvas.fill_path(
                &path,
                &shadow_paint,
                FillRule::Winding,
                shadow_transform,
                None,
            );
            p.canvas
                .fill_path(&path, &fill_paint, FillRule::Winding, transform, None);
        }
    }

    if let Some(circle) = oval_path(start.0, start.1, circle_radius, circle_radius) {
        p.canvas.fill_path(
            &circle,
            &shadow_paint,
            FillRule::Winding,
            shadow_transform,
            None,
        );
        p.canvas
            .fill_path(&circle, &fill_paint, FillRule::Winding, transform, None);
    }

    let digits = number.to_string().len();

    let font_size = circle_radius * numbered.digit_sizes[digits.clamp(1, 4) - 1];
    let weight = if numbered.bold {
        cosmic_text::Weight::BOLD
    } else {
        cosmic_text::Weight::NORMAL
    };

    let Some(fonts) = p.fonts.as_mut() else {
        return;
    };
    let (buffer, text_width, text_height) = shape_single_line(
        fonts.system,
        &number.to_string(),
        font_size,
        weight,
        cosmic_text::Style::Normal,
    );

    let text_pos = (start.0 - text_width / 2.0, start.1 - text_height / 2.0);

    let text_color = contrasting_text_color(color);

    p.text_buffer(&buffer, text_pos, text_color);
}
