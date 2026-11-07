//! 【文字识别】【原图绘制】只分配可见视区缓冲，保留原图透明度，不添加 Qt 没有的棋盘背景

use tiny_skia::{FilterQuality, Pixmap, PixmapPaint, Rect, Transform};

use super::source_preview::SourcePreview;

/// 【文字识别】【裁切原图】把 image 按 preview 的矩形绘制到 canvas 的 body，scale 为缓冲比例，无返回值；放大内容不越过视区
pub(super) fn draw(
    preview: &mut SourcePreview,
    image: &Pixmap,
    canvas: &mut Pixmap,
    body: Rect,
    scale: f32,
) {
    let viewport = (body.width() as i32, body.height() as i32);
    let Some([x, y, width, height]) = preview.rect(viewport, (image.width(), image.height()))
    else {
        return;
    };
    let size = (
        (body.width() * scale).ceil() as u32,
        (body.height() * scale).ceil() as u32,
    );
    if preview
        .raster
        .as_ref()
        .is_none_or(|raster| (raster.width(), raster.height()) != size)
    {
        preview.raster = Pixmap::new(size.0, size.1);
    }
    let Some(raster) = &mut preview.raster else {
        return;
    };
    // 1. 【文字识别】【视区缓冲】透明底板让原图透明像素透出父背景，缓冲容量与放大倍数无关
    raster.fill(tiny_skia::Color::TRANSPARENT);
    raster.draw_pixmap(
        0,
        0,
        image.as_ref(),
        &PixmapPaint {
            quality: FilterQuality::Bilinear,
            ..PixmapPaint::default()
        },
        Transform::from_scale(
            width as f32 / image.width() as f32 * scale,
            height as f32 / image.height() as f32 * scale,
        )
        .post_translate(x as f32 * scale, y as f32 * scale),
        None,
    );
    // 2. 【文字识别】【父窗口合成】只合成已裁切视区，不覆盖标题、页签或底部反馈
    canvas.draw_pixmap(
        (body.x() * scale).round() as i32,
        (body.y() * scale).round() as i32,
        raster.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}
