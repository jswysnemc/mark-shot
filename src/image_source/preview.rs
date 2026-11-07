//! 【图像原件】【分批预览】借用完整原件坐标，按完整输出行分段绘制，不复制原件或创建整屏遮罩

use super::{ImageSource, MEMORY_BYTES};
use tiny_skia::{
    FillRule, IntRect, Paint, PathBuilder, Pattern, Pixmap, PixmapPaint, Rect, SpreadMode,
    Transform,
};

/// 【图像原件】【预览行带】按source、target、paint和transform绘制原始预乘像素，行带完成后归还原件读取页，无返回值
pub(super) fn draw(
    source: &ImageSource,
    target: &mut Pixmap,
    paint: &PixmapPaint,
    transform: Transform,
    clip: Option<IntRect>,
) {
    if transform.kx != 0.0 || transform.ky != 0.0 {
        target.draw_pixmap(0, 0, source.as_ref(), paint, transform, None);
        source.release_pages();
        return;
    }
    let Some(mut bounds) = Rect::from_xywh(0.0, 0.0, source.width() as f32, source.height() as f32)
        .and_then(|rect| rect.transform(transform))
    else {
        return;
    };
    if let Some(clip) = clip {
        let Some(clipped) = bounds.intersect(&clip.to_rect()) else {
            return;
        };
        bounds = clipped;
    }
    let width = target.width();
    let height = target.height() as usize;
    let stride = width as usize * 4;
    // 1. 【图像原件】【读取预算】源图缩小时同时限制对应原件行数，避免单个输出行带读取完整大图
    let source_stride = source.width() as usize * 4;
    let source_rows = (MEMORY_BYTES / source_stride).max(1);
    let scale = transform.sy.abs().max(f32::MIN_POSITIVE);
    let rows = ((source_rows as f32 * scale).floor() as usize)
        .max(1)
        .min((MEMORY_BYTES / stride).max(1));
    let first = bounds.top().floor().max(0.0) as usize;
    let last = (bounds.bottom().ceil().max(0.0) as usize).min(height);
    for start in (first..last).step_by(rows) {
        let end = (start + rows).min(last);
        let Some(band) = Rect::from_xywh(0.0, start as f32, width as f32, (end - start) as f32)
            .and_then(|rect| rect.intersect(&bounds))
        else {
            continue;
        };
        // 2. 【图像原件】【原始采样】保留完整画布坐标与原始采样变换，仅限制输出行带，不改变逆变换的浮点计算
        let batch_paint = Paint {
            shader: Pattern::new(
                source.as_ref(),
                SpreadMode::Pad,
                paint.quality,
                paint.opacity,
                transform,
            ),
            blend_mode: paint.blend_mode,
            anti_alias: false,
            ..Paint::default()
        };
        if transform.is_identity() {
            target.fill_rect(band, &batch_paint, Transform::identity(), None);
        } else {
            // 3. 【图像原件】【矩形栅格】原有非恒等变换走路径扫描，分批绘制保留同一边缘扫描规则
            target.fill_path(
                &PathBuilder::from_rect(band),
                &batch_paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        source.release_pages();
    }
}
