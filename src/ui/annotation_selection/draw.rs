//! 【标注选框】【统一绘制】原版橙色完整虚线、方块缩放点、圆形旋转点与红色删除按钮

use crate::{
    features::annotation_selection::chrome::{self, Point},
    renderer::Painter,
};
use tiny_skia::{Color, LineCap, LineJoin, Path, PathBuilder, Rect, Stroke, StrokeDash};

/// 【标注选框】【完整描边】把 path 绘入 painter，unit 为逻辑样式比例，无返回值，不添加旧式黑色光晕
pub fn outline(painter: &mut Painter<'_>, path: &Path, unit: f32) {
    let unit = unit * painter.chrome_scale();
    painter.stroke_path(
        path,
        chrome::color(),
        &Stroke {
            width: 2.0 * unit,
            line_cap: LineCap::Square,
            line_join: LineJoin::Bevel,
            dash: StrokeDash::new(vec![8.0 * unit, 4.0 * unit], 0.0),
            ..Default::default()
        },
    );
}

/// 【标注选框】【矩形外框】按 corners 绘制闭合四边形，unit 控制样式比例，无返回值
pub fn frame(painter: &mut Painter<'_>, corners: [Point; 4], unit: f32) {
    let mut path = PathBuilder::new();
    path.move_to(corners[0].0, corners[0].1);
    for point in &corners[1..] {
        path.line_to(point.0, point.1);
    }
    path.close();
    if let Some(path) = path.finish() {
        outline(painter, &path, unit);
    }
}

/// 【标注选框】【方块缩放点】按 point 与 unit 绘制9像素、2像素圆角橙色方块，无返回值
pub fn square(painter: &mut Painter<'_>, point: Point, unit: f32) {
    let unit = unit * painter.chrome_scale();
    if let Some(rect) = Rect::from_xywh(
        point.0 - 4.5 * unit,
        point.1 - 4.5 * unit,
        9.0 * unit,
        9.0 * unit,
    ) {
        super::control_raster::draw(painter, rect, |painter| {
            painter.fill_rounded(rect, 2.0 * unit, chrome::color());
        });
    }
}

/// 【标注选框】【圆形控制点】按 point、radius、unit、fill、border 与 width 绘制独立定点，无返回值
pub fn circle(
    painter: &mut Painter<'_>,
    point: Point,
    radius: f32,
    unit: f32,
    fill: Color,
    border: Color,
    width: f32,
) {
    let unit = unit * painter.chrome_scale();
    let extent = (radius + width * 0.5) * unit + 1.0;
    let Some(bounds) = Rect::from_xywh(
        point.0 - extent,
        point.1 - extent,
        extent * 2.0,
        extent * 2.0,
    ) else {
        return;
    };
    super::control_raster::draw(painter, bounds, |painter| {
        painter.fill_circle(point.0, point.1, radius * unit, fill);
        let mut path = PathBuilder::new();
        path.push_circle(point.0, point.1, radius * unit);
        if let Some(path) = path.finish() {
            painter.stroke_path(
                &path,
                border,
                &Stroke {
                    width: width * unit,
                    ..Default::default()
                },
            );
        }
    });
}

/// 【标注选框】【旋转控制点】按start、end和unit绘制连接线及12像素圆点，导航反向比例保持窗口间隔，无返回值
pub fn rotation(painter: &mut Painter<'_>, start: Point, end: Point, unit: f32) {
    let end = crate::features::annotation_selection::rotation::projected(
        start,
        end,
        painter.chrome_scale(),
    );
    rotation_points(painter, start, end, unit);
}

/// 【标注选框】【窗口旋转绘制】按start、end和unit绘制已经完成窗口换算的连线，圆点尺寸仍使用绘制器窗口比例，无返回值
pub fn rotation_points(painter: &mut Painter<'_>, start: Point, end: Point, unit: f32) {
    let mut path = PathBuilder::new();
    path.move_to(start.0, start.1);
    path.line_to(end.0, end.1);
    if let Some(path) = path.finish() {
        painter.stroke_path(
            &path,
            chrome::color(),
            &Stroke {
                width: 1.8 * unit * painter.chrome_scale(),
                line_cap: LineCap::Round,
                ..Default::default()
            },
        );
    }
    circle(painter, end, 6.0, unit, chrome::color(), Color::WHITE, 1.5);
}

/// 【标注选框】【通用控制框】按 rect 绘制完整外框和八个缩放点，unit 控制尺寸，无返回值
pub fn rectangle(painter: &mut Painter<'_>, rect: Rect, unit: f32) {
    let corners = chrome::corners(rect);
    frame(painter, corners, unit);
    for point in chrome::handles(corners) {
        square(painter, point, unit);
    }
}

/// 【标注选框】【删除按钮】按 rect 绘制原版红色圆按钮与白色叉号，无返回值，按钮不属于图像导出内容
pub fn delete(painter: &mut Painter<'_>, rect: Rect) {
    let center = chrome::center(rect);
    circle(
        painter,
        center,
        10.0,
        1.0,
        Color::from_rgba8(239, 68, 68, 255),
        Color::WHITE,
        2.0,
    );
    let mut path = PathBuilder::new();
    path.move_to(center.0 - 4.5, center.1 - 4.5);
    path.line_to(center.0 + 4.5, center.1 + 4.5);
    path.move_to(center.0 + 4.5, center.1 - 4.5);
    path.line_to(center.0 - 4.5, center.1 + 4.5);
    if let Some(path) = path.finish() {
        painter.stroke_path(
            &path,
            Color::WHITE,
            &Stroke {
                width: 2.0,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Default::default()
            },
        );
    }
}
