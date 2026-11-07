//! 【尺子】【叠加绘制】最后绘制测量说明、临时十字线与选中控制点

use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::{Color, PathBuilder, Pixmap, Rect, Stroke, StrokeDash};

use super::{Painter, rect_bounds, scale_rect};
use crate::features::ruler::RulerGeometry;
use crate::features::ruler::drag;
use crate::features::ruler::labels::InfoLabel;
use crate::features::ruler::state::RulerHover;
use crate::types::{Annotation, AnnotationShape};

/// 【尺子】【全部说明绘制】按顺序把 labels 绘入 painter；导出在全部持久标注正文之后调用，无返回值
pub fn draw_labels(painter: &mut Painter<'_>, labels: &[InfoLabel]) {
    super::ruler_labels::draw(painter, labels);
}

/// 【尺子】【悬停绘制】将 hover 的临时十字线与原图坐标、颜色绘入 painter；不用于导出，无返回值
pub(super) fn draw_hover(painter: &mut Painter<'_>, hover: &RulerHover) {
    let mut path = PathBuilder::new();
    path.move_to(hover.bounds.left(), hover.point.1);
    path.line_to(hover.bounds.right(), hover.point.1);
    path.move_to(hover.point.0, hover.bounds.top());
    path.line_to(hover.point.0, hover.bounds.bottom());
    if let Some(path) = path.finish() {
        painter.stroke_path(
            &path,
            crate::theme::toolbar::Palette::current()
                .hover_fill
                .fade(150.0 / 255.0),
            &Stroke {
                width: 1.0,
                dash: StrokeDash::new(vec![6.0, 4.0], 0.0),
                ..Stroke::default()
            },
        );
    }
    super::ruler_hover_label::draw(
        painter,
        &hover.label,
        crate::theme::toolbar::Palette::current(),
    );
}

/// 【尺子】【局部叠加】把 labels 和可选 hover 绘入 canvas；origin、scale 映射坐标，dirty 限制局部区域，fonts、cache 提供字体，无返回值
pub fn draw_clipped(
    canvas: &mut Pixmap,
    labels: &[InfoLabel],
    hover: Option<&RulerHover>,
    origin: (f32, f32),
    scale: f32,
    dirty: Option<&Rect>,
    fonts: &mut FontSystem,
    cache: &mut SwashCache,
) {
    draw_clipped_with_window(
        canvas,
        labels,
        hover,
        origin,
        scale,
        dirty,
        fonts,
        cache,
        Default::default(),
    );
}

/// 【尺子】【窗口叠加】按完整窗口单位和其他绘制参数合成labels及hover，说明字号独立于设备比例，无返回值
pub(crate) fn draw_clipped_with_window(
    canvas: &mut Pixmap,
    labels: &[InfoLabel],
    hover: Option<&RulerHover>,
    origin: (f32, f32),
    scale: f32,
    dirty: Option<&Rect>,
    fonts: &mut FontSystem,
    cache: &mut SwashCache,
    window: crate::features::annotation_style::window::WindowUnits,
) {
    if labels.is_empty() && hover.is_none() {
        return;
    }
    if let Some(hover) = hover {
        super::ruler_hover_tiles::draw(
            canvas, labels, hover, origin, scale, dirty, fonts, cache, window,
        );
        return;
    }
    let Some(area) = dirty
        .copied()
        .and_then(|rect| scale_rect(&rect, scale))
        .or_else(|| Rect::from_xywh(0.0, 0.0, canvas.width() as f32, canvas.height() as f32))
    else {
        return;
    };
    let Some((mut x, mut y, mut width, mut height)) =
        rect_bounds(&area, canvas.width(), canvas.height())
    else {
        return;
    };
    let local_origin = (origin.0 + x as f32 / scale, origin.1 + y as f32 / scale);
    let Some(global) = Rect::from_xywh(
        local_origin.0,
        local_origin.1,
        width as f32 / scale,
        height as f32 / scale,
    ) else {
        return;
    };
    let visible = |item: &&InfoLabel| item.rect.intersect(&global).is_some();
    if !labels.iter().any(|item| visible(&item)) && hover.is_none() {
        return;
    }
    // 1. 【尺子】【说明缓冲范围】没有整幅十字线时，只合成可见说明及字形抗锯齿留量，不分配整幅窗口缓冲
    if hover.is_none() {
        let content = labels
            .iter()
            .filter(visible)
            .filter_map(|item| item.rect.outset(window.canvas(2.0), window.canvas(2.0)))
            .reduce(|first, second| first.join(&second).unwrap_or(first));
        let clipped = content
            .and_then(|rect| super::pixel_bounds::map(rect, origin, f64::from(scale)))
            .and_then(|rect| {
                rect.intersect(&tiny_skia::IntRect::from_xywh(
                    x as i32, y as i32, width, height,
                )?)
            });
        let Some(clipped) = clipped else { return };
        x = clipped.x() as u32;
        y = clipped.y() as u32;
        width = clipped.width();
        height = clipped.height();
    }
    let local_origin = (origin.0 + x as f32 / scale, origin.1 + y as f32 / scale);
    // 2. 【尺子】【局部叠加】说明背景和十字线半透明，只在刚恢复的区域绘制一次
    let Some(mut temporary) = Pixmap::new(width, height) else {
        return;
    };
    let mut painter = Painter::new(&mut temporary)
        .with_scale(scale)
        .with_window_units(window)
        .with_origin(local_origin)
        .with_text(fonts, cache);
    super::ruler_labels::draw(&mut painter, labels);
    if let Some(hover) = hover {
        draw_hover(&mut painter, hover);
    }
    // 3. 【尺子】【局部叠加】只合成损坏范围，未清除像素不重复叠加背景或文字
    Painter::new(canvas).draw_pixmap(x as i32, y as i32, &temporary);
}

/// 【尺子】【选中控制框】按 annotation 绘制尺子操作框、旋转把手与定点；不修改测量数据，无返回值
pub(crate) fn draw_handles(painter: &mut Painter<'_>, annotation: &Annotation) {
    let AnnotationShape::Ruler { measurement } = annotation.shape else {
        return;
    };
    let window = painter.window_units();
    let Some(body) = measurement
        .bounds()
        .and_then(|body| {
            if measurement.mode() == crate::features::ruler::RulerMode::Angle {
                drag::window_frame(measurement, window)
            } else {
                Some(body)
            }
        })
        .and_then(|body| body.outset(window.canvas(8.0), window.canvas(8.0)))
    else {
        return;
    };
    let handle = super::ruler_chrome::connector(painter, annotation).and_then(|(_, end)| {
        Rect::from_xywh(
            end.0 - window.canvas(8.0),
            end.1 - window.canvas(8.0),
            window.canvas(16.0),
            window.canvas(16.0),
        )
    });
    let bounds = handle.and_then(|handle| body.join(&handle)).unwrap_or(body);
    // 1. 【尺子】【选中栅格】旋转虚线与连线共用固定物理网格，局部缓冲只改变最后合成的交集
    super::rotation_tiles::draw_region(painter, bounds, |painter| {
        draw_handles_content(painter, annotation)
    });
}

/// 【尺子】【控制框内容】按painter和annotation绘制完整选中框，调用方负责固定物理网格，无返回值
fn draw_handles_content(painter: &mut Painter<'_>, annotation: &Annotation) {
    use crate::{features::annotation_selection::chrome, ui::annotation_selection::draw};
    let AnnotationShape::Ruler { measurement } = annotation.shape else {
        return;
    };
    let window = painter.window_units();
    let Some(frame) = drag::window_frame(measurement, window) else {
        return;
    };
    let color = chrome::color();
    let mut path = PathBuilder::new();
    if let Some(corners) = measurement.corners() {
        path.move_to(corners[0].0, corners[0].1);
        for point in &corners[1..] {
            path.line_to(point.0, point.1);
        }
        path.close();
    } else {
        path.push_rect(frame);
    }
    if let Some(path) = path.finish() {
        draw::outline(painter, &path, 1.0);
    }
    if let Some((top, handle)) = super::ruler_chrome::connector(painter, annotation) {
        draw::rotation_points(painter, top, handle, 1.0);
    }
    if matches!(measurement.geometry, RulerGeometry::Angle { .. }) {
        for (point, _) in drag::frame_handles(frame) {
            draw::square(painter, point, 1.0);
        }
    }
    for (point, _) in drag::handles(measurement, window.canvas(drag::HANDLE_TOLERANCE)) {
        if matches!(measurement.geometry, RulerGeometry::Angle { .. }) {
            draw::circle(painter, point, 5.5, 1.0, color, Color::WHITE, 1.5);
        } else {
            draw::square(painter, point, 1.0);
        }
    }
}
