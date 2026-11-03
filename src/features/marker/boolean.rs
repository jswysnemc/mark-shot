//! 【形状标记】【轮廓运算】用浮点布尔运算合并轮廓和保留孔洞，不启用额外线程

use i_overlay::core::{fill_rule::FillRule, overlay_rule::OverlayRule};
use i_overlay::float::single::SingleFloatOverlay;
use kurbo::{PathEl, Point};
use tiny_skia::{Path, PathBuilder, PathSegment};

type Contours = Vec<Vec<[f64; 2]>>;
type Shapes = Vec<Contours>;

/// 【形状标记】【曲线展平】把 path 展平为闭合单位轮廓；返回布尔运算坐标，单位长度展平容差设为 0.00001
fn contours(path: &Path) -> Contours {
    let point = |point: tiny_skia::Point| Point::new(point.x as f64, point.y as f64);
    let elements = path.segments().map(|segment| match segment {
        PathSegment::MoveTo(p) => PathEl::MoveTo(point(p)),
        PathSegment::LineTo(p) => PathEl::LineTo(point(p)),
        PathSegment::QuadTo(control, end) => PathEl::QuadTo(point(control), point(end)),
        PathSegment::CubicTo(a, b, end) => PathEl::CurveTo(point(a), point(b), point(end)),
        PathSegment::Close => PathEl::ClosePath,
    });
    let mut result = Vec::new();
    let mut current = Vec::new();
    kurbo::flatten(elements, 0.00001, |element| match element {
        PathEl::MoveTo(point) => {
            finish_contour(&mut result, &mut current);
            current.push([point.x, point.y]);
        }
        PathEl::LineTo(point) => current.push([point.x, point.y]),
        PathEl::ClosePath => finish_contour(&mut result, &mut current),
        PathEl::QuadTo(..) | PathEl::CurveTo(..) => {
            unreachable!("flatten emits only line segments")
        }
    });
    finish_contour(&mut result, &mut current);
    result
}

/// 【形状标记】【闭合轮廓】去除 current 中的重复终点并移入 result；无返回值
fn finish_contour(result: &mut Contours, current: &mut Vec<[f64; 2]>) {
    if current.len() > 1 && current.first() == current.last() {
        current.pop();
    }
    if current.len() >= 3 {
        result.push(std::mem::take(current));
    } else {
        current.clear();
    }
}

/// 【形状标记】【结果路径】将 shapes 中的外轮廓与反向孔洞还原为路径；返回可按非零规则填充的路径
fn path(shapes: Shapes) -> Option<Path> {
    let mut builder = PathBuilder::new();
    for contour in shapes.into_iter().flatten() {
        let Some(first) = contour.first() else {
            continue;
        };
        builder.move_to(first[0] as f32, first[1] as f32);
        for point in &contour[1..] {
            builder.line_to(point[0] as f32, point[1] as f32);
        }
        builder.close();
    }
    builder.finish()
}

/// 【形状标记】【轮廓合并】将 parts 做区域并集，消除重叠位置的内部描边；返回闭合路径
pub(super) fn unite(parts: &[Path]) -> Option<Path> {
    let first = parts.first()?;
    let mut result = vec![contours(first)];
    for part in &parts[1..] {
        result = result.overlay(&contours(part), OverlayRule::Union, FillRule::NonZero);
    }
    path(result)
}

/// 【形状标记】【轮廓挖孔】从 outer 中减去 hole；返回带反向孔洞或裁切边界的路径
pub(super) fn subtract(outer: &Path, hole: &Path) -> Option<Path> {
    path(contours(outer).overlay(&contours(hole), OverlayRule::Difference, FillRule::NonZero))
}

/// 【形状标记】【绕向统一】统一 source 的外轮廓与孔洞方向；返回非零填充规则下的等价路径
pub(super) fn normalize(source: &Path) -> Option<Path> {
    let empty: Contours = Vec::new();
    path(contours(source).overlay(&empty, OverlayRule::Union, FillRule::NonZero))
}

/// 【形状标记】【路径组合】直接组合 parts，保留原版禁止标记的各条描边；返回闭合路径
pub(super) fn join(parts: &[Path]) -> Option<Path> {
    let mut builder = PathBuilder::new();
    for part in parts {
        builder.push_path(part);
    }
    builder.finish()
}
