//! 【形状标记】【路径缓存】按需生成单位轮廓，并映射到逻辑或导出矩形

use std::f32::consts::{FRAC_PI_2, PI};
use std::sync::OnceLock;

use tiny_skia::{Path, Rect, Transform};

use super::{composite, curves, primitives as primitive};
use crate::types::MarkerShape;

static PATHS: [OnceLock<Option<Path>>; 24] = [const { OnceLock::new() }; 24];

/// 【形状标记】【路径映射】将 shape 的单位轮廓映射到 rect；空矩形返回空值，首次使用才生成路径
pub fn path_for_shape(shape: MarkerShape, rect: Rect) -> Option<Path> {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return None;
    }
    let path = PATHS[shape as usize]
        .get_or_init(|| unit_path(shape))
        .as_ref()?;
    path.clone().transform(
        Transform::from_scale(rect.width(), rect.height()).post_translate(rect.left(), rect.top()),
    )
}

/// 【形状标记】【单位轮廓】根据 shape 生成原版几何形状；返回单位矩形内的闭合路径
fn unit_path(shape: MarkerShape) -> Option<Path> {
    match shape {
        MarkerShape::Triangle => primitive::polygon(&[(0.50, 0.10), (0.90, 0.88), (0.10, 0.88)]),
        MarkerShape::Star => primitive::star(5, 0.45, 0.18),
        MarkerShape::Check => {
            primitive::round_line(&[(0.16, 0.54), (0.40, 0.76), (0.86, 0.24)], 0.17)
        }
        MarkerShape::Cross => composite::cross(),
        MarkerShape::Diamond => {
            primitive::polygon(&[(0.50, 0.08), (0.90, 0.50), (0.50, 0.92), (0.10, 0.50)])
        }
        MarkerShape::Heart => curves::heart(),
        MarkerShape::Hexagon => primitive::regular_polygon(6, 0.45, -FRAC_PI_2),
        MarkerShape::Circle => primitive::circle(0.50, 0.50, 0.40),
        MarkerShape::Square => {
            primitive::rounded_rectangle(Rect::from_xywh(0.14, 0.14, 0.72, 0.72)?, 0.08)
        }
        MarkerShape::Pentagon => primitive::regular_polygon(5, 0.45, -FRAC_PI_2),
        MarkerShape::Plus => primitive::polygon(&[
            (0.40, 0.14),
            (0.60, 0.14),
            (0.60, 0.40),
            (0.86, 0.40),
            (0.86, 0.60),
            (0.60, 0.60),
            (0.60, 0.86),
            (0.40, 0.86),
            (0.40, 0.60),
            (0.14, 0.60),
            (0.14, 0.40),
            (0.40, 0.40),
        ]),
        MarkerShape::ArrowUp => primitive::polygon(&[
            (0.50, 0.10),
            (0.84, 0.46),
            (0.66, 0.46),
            (0.66, 0.88),
            (0.34, 0.88),
            (0.34, 0.46),
            (0.16, 0.46),
        ]),
        MarkerShape::Spade => curves::spade(),
        MarkerShape::Club => composite::club(),
        MarkerShape::Lightning => primitive::polygon(&[
            (0.62, 0.06),
            (0.24, 0.54),
            (0.44, 0.54),
            (0.36, 0.94),
            (0.78, 0.42),
            (0.56, 0.42),
        ]),
        MarkerShape::Ban => composite::ban(),
        MarkerShape::Octagon => primitive::regular_polygon(8, 0.45, PI / 8.0),
        MarkerShape::Crescent => composite::crescent(),
        MarkerShape::Pin => composite::pin(),
        MarkerShape::Flag => composite::flag(),
        MarkerShape::Bookmark => curves::bookmark(),
        MarkerShape::Shield => curves::shield(),
        MarkerShape::Exclamation => composite::exclamation(),
        MarkerShape::SpeechBubble => composite::speech_bubble(),
    }
}
