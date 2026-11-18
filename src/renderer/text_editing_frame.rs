//! 【文字编辑】【输入外框】动态虚线标出实际输入范围，不进入正文缓存和图片导出

use super::{Painter, RenderRequest};
use crate::{theme::Rgba, types::AnnotationShape};
use tiny_skia::{Rect, Stroke, StrokeDash};

/// 【文字编辑】【外框绘制】按 req、origin、scale、window_unit 和 dirty 绘制活动正文外框，已有选择框不重复描边，无返回值
pub(super) fn draw(
    req: &mut RenderRequest<'_>,
    origin: (f32, f32),
    scale: f32,
    window_unit: f32,
    dirty: Option<Rect>,
) {
    let Some(annotation) = req
        .pending
        .filter(|annotation| req.active_text_id == Some(annotation.id) && !req.is_pending_selected)
    else {
        return;
    };
    let AnnotationShape::Text {
        content,
        appearance,
        ..
    } = &annotation.shape
    else {
        return;
    };
    let editor =
        crate::tools::text::ensure_text_editor(annotation, req.text.editors, req.text.font_system);
    let mut bounds = crate::tools::text::text_bounds(annotation, editor);
    if content.is_empty() {
        bounds = Rect::from_xywh(
            bounds.left(),
            bounds.top(),
            bounds.width() + window_unit,
            bounds.height(),
        )
        .unwrap();
    }
    // 1. 【文字编辑】【原版尺寸】编辑框向内收半个窗口像素，圆角随标注尺寸，描边保持一个物理像素
    let inset = 0.5 * window_unit;
    let Some(bounds) = bounds.inset(inset, inset) else {
        return;
    };
    let Some(path) = super::paths::rounded_rect_path(
        &bounds,
        4.0 * appearance.size_scale,
        true,
        true,
        true,
        true,
    ) else {
        return;
    };
    let width = 1.0 / scale.max(0.1);
    let stroke = Stroke {
        width,
        line_cap: tiny_skia::LineCap::Square,
        dash: StrokeDash::new(vec![4.0 * width, 2.0 * width], 0.0),
        ..Default::default()
    };
    let transform =
        tiny_skia::Transform::from_translate(-origin.0, -origin.1).post_scale(scale, scale);
    let Some(outline) = path
        .stroke(&stroke, scale)
        .and_then(|path| path.transform(transform))
    else {
        return;
    };
    let pad = (4.0 * appearance.size_scale).max(width) + width;
    let interior = bounds
        .inset(pad, pad)
        .and_then(|bounds| bounds.transform(transform));
    let dirty = dirty.and_then(|rect| super::scale_rect(&rect, scale));
    // 2. 【文字编辑】【固定栅格】先生成完整物理描边，再按固定块裁切，局部重绘不会改变虚线采样或叠加透明度
    Painter::new(req.canvas).clipped(dirty, |painter| {
        super::fixed_path::fill_border(
            painter,
            &outline,
            Rgba(45, 212, 191, 210).color(),
            interior,
        );
    });
}
