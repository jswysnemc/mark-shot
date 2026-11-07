//! 【标注多选】【局部绘制】共同手势保持原始层次，群组成员和联合控制框只绘入屏幕动态层

use crate::{
    app::annotation_collection::chrome::Overlay,
    features::{annotation_collection::geometry, annotation_selection::chrome},
    renderer::{Painter, RenderRequest},
    ui::annotation_selection::draw,
};
use tiny_skia::{Color, LineCap, LineJoin, Rect, Stroke, StrokeDash};

/// 【标注多选】【正文合成】将req全部当前持久成员按原始图层顺序合成，损坏区域外跳过，无返回值
pub(super) fn bodies(req: &mut RenderRequest<'_>) {
    super::annotation_stack::draw_ordered(
        &mut Painter::new(req.canvas)
            .with_scale(req.scale)
            .with_origin(req.offset)
            .with_frozen_image(req.frozen_image)
            .with_drawing_bounds(req.drawing_bounds)
            .with_text(req.text.font_system, req.text.swash_cache),
        req.annotations.iter(),
        req.text.editors,
        req.filters,
        req.active_text_id,
        req.dirty_rect.copied(),
    );
}

/// 【标注多选】【圆角虚线】按painter、bounds、radius、color与width绘制Qt虚线，虚线长度按各自笔宽计算，无返回值
fn dashed(painter: &mut Painter<'_>, bounds: Rect, radius: f32, color: Color, width: f32) {
    let unit = painter.chrome_scale();
    let width = width * unit;
    let radius = radius * unit;
    let Some(path) = super::rounded_rect_path(&bounds, radius, true, true, true, true) else {
        return;
    };
    let mut stroke = Stroke {
        width,
        line_cap: LineCap::Square,
        line_join: LineJoin::Bevel,
        dash: StrokeDash::new(vec![4.0 * width, 2.0 * width], 0.0),
        ..Default::default()
    };
    let Some(dash) = stroke.dash.take() else {
        return;
    };
    let Some(outline) = path
        .dash(&dash, painter.scale())
        .and_then(|path| path.stroke(&stroke, painter.scale()))
    else {
        return;
    };
    // 1. 【标注多选】【固定描边】完整虚线先转换为填充轮廓，再以固定像素原点分块合成，条带裁切不改变圆角采样
    super::fixed_path::fill(painter, &outline, color);
}

/// 【标注多选】【外框合成】按req与overlay绘制成员白底虚线、联合框、八个缩放点和旋转点，不进入图像导出，无返回值
pub(super) fn chrome(req: &mut RenderRequest<'_>, overlay: &Overlay<'_>) {
    chrome_with_view(req, overlay, Default::default());
}

/// 【画布导航】【群组控件】按req、overlay与view绘制变换后的共同外框，控件尺寸维持屏幕逻辑单位，无返回值
pub(super) fn chrome_with_view(
    req: &mut RenderRequest<'_>,
    overlay: &Overlay<'_>,
    view: crate::editor::canvas_navigation::transform::ViewTransform,
) {
    let unit = 1.0 / view.scale as f32;
    let dirty = req.dirty_rect.and_then(|rect| {
        Rect::from_xywh(
            rect.x() * unit,
            rect.y() * unit,
            rect.width() * unit,
            rect.height() * unit,
        )
    });
    let mut painter = Painter::new(req.canvas)
        .with_scale(req.scale * view.scale as f32)
        .with_origin(view.origin(req.offset))
        .with_chrome_scale(unit);
    painter.clipped(dirty, |painter| {
        if overlay.ids.len() > 1
            && let Some(bounds) = overlay.bounds
        {
            // 1. 【标注多选】【成员外框】白色底线与橙色细线按生产绘制顺序分别完成全部成员
            for (color, width) in [
                (Color::from_rgba8(255, 255, 255, 190), 3.0),
                (Color::from_rgba8(251, 146, 60, 170), 1.0),
            ] {
                for annotation in req
                    .annotations
                    .iter()
                    .filter(|annotation| overlay.ids.contains(&annotation.id))
                {
                    if let Some(bounds) = geometry::visible_bounds(annotation, overlay.canvas) {
                        dashed(painter, bounds, 3.0, color, width);
                    }
                }
            }
            dashed(
                painter,
                bounds,
                4.0,
                Color::from_rgba8(255, 255, 255, 230),
                4.0,
            );
            dashed(painter, bounds, 4.0, chrome::color(), 2.0);
            for point in chrome::handles(chrome::corners(bounds)) {
                let outer = Rect::from_xywh(
                    point.0 - 5.8 * unit,
                    point.1 - 5.8 * unit,
                    11.6 * unit,
                    11.6 * unit,
                )
                .unwrap();
                crate::ui::annotation_selection::control_raster::draw(
                    painter,
                    outer.outset(unit, unit).unwrap(),
                    |painter| {
                        painter.fill_rounded(outer, 2.6 * unit, Color::WHITE);
                        painter.stroke_path(
                            &super::rounded_rect_path(&outer, 2.6 * unit, true, true, true, true)
                                .unwrap(),
                            Color::WHITE,
                            &Stroke {
                                width: 1.5 * unit,
                                ..Default::default()
                            },
                        );
                    },
                );
                draw::square(painter, point, 1.0);
            }
            let (top, rotate) = geometry::rotation_handle(bounds);
            draw::rotation(painter, top, rotate, 1.0);
        }
        // 2. 【标注多选】【框选外框】青色填充和虚线仅表示本次相交选择范围，提交后清除
        if let Some(bounds) = overlay.marquee {
            painter.fill_rounded(bounds, 4.0, Color::from_rgba8(45, 212, 191, 34));
            dashed(
                painter,
                bounds,
                4.0,
                Color::from_rgba8(45, 212, 191, 255),
                1.5,
            );
        }
    });
}
