//! 【滚动截图】【捕获范围】黄色框独立表达最近捕获帧，不随细节浏览位置移动

use crate::{renderer::Painter, theme::Rgba};
use tiny_skia::Rect;

/// 【滚动截图】【范围映射】按overview、length、horizontal与range返回裁切后的捕获框，未匹配或范围外返回空值
pub(super) fn rect(
    overview: Rect,
    length: f32,
    horizontal: bool,
    range: (i32, i32),
) -> Option<Rect> {
    if !length.is_finite() || length <= 0.0 || range.1 <= 0 {
        return None;
    }
    let start = (range.0 as f32).clamp(0.0, length);
    let end = ((i64::from(range.0) + i64::from(range.1)) as f32).clamp(0.0, length);
    if end <= start {
        return None;
    }
    let (x, y, width, height) = if horizontal {
        (
            overview.left() + (overview.width() * start / length).round(),
            overview.top(),
            (overview.width() * (end - start) / length).round().max(3.0),
            overview.height(),
        )
    } else {
        (
            overview.left(),
            overview.top() + (overview.height() * start / length).round(),
            overview.width(),
            (overview.height() * (end - start) / length)
                .round()
                .max(3.0),
        )
    };
    Rect::from_xywh(x, y, width, height)?.intersect(&overview)
}

/// 【滚动截图】【黄色标记】按捕获原图区间绘制原版两像素黄色边框及透明填充，不分配长图缓冲
pub(super) fn draw(
    p: &mut Painter,
    overview: Rect,
    length: f32,
    horizontal: bool,
    range: (i32, i32),
) {
    if let Some(marker) = rect(overview, length, horizontal, range) {
        p.fill_rounded(marker, 0.0, Rgba(250, 204, 21, 42));
        p.stroke_path(
            &tiny_skia::PathBuilder::from_rect(marker),
            Rgba(250, 204, 21, 245),
            &tiny_skia::Stroke {
                width: 2.0,
                ..Default::default()
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【滚动截图测试】【双向范围】无参数，纵横映射、负起点和越界区间保持有效概览范围，无返回值
    #[test]
    fn ranges_follow_source_coordinates_and_clip_to_overview() {
        let area = Rect::from_xywh(10.0, 20.0, 100.0, 200.0).unwrap();
        assert_eq!(
            rect(area, 1000.0, false, (400, 200)).unwrap(),
            Rect::from_xywh(10.0, 100.0, 100.0, 40.0).unwrap()
        );
        assert_eq!(
            rect(area, 1000.0, true, (400, 200)).unwrap(),
            Rect::from_xywh(50.0, 20.0, 20.0, 200.0).unwrap()
        );
        assert_eq!(
            rect(area, 1000.0, false, (-50, 100)).unwrap().height(),
            10.0
        );
        assert!(rect(area, 1000.0, false, (1100, 100)).is_none());
        assert!(rect(area, 0.0, false, (0, 100)).is_none());
    }
}
