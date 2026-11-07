//! 【选区尺寸】【面板避让】依次尝试选区左上外侧、左上内侧和左下外侧，必要时由应用放宽障碍物

use tiny_skia::Rect;

/// 【选区尺寸】【尺寸面板位置】按 selection、size、bounds 和 obstacles 返回第一个完整可用位置，全部冲突返回空值
pub fn hud(selection: Rect, size: (f32, f32), bounds: Rect, obstacles: &[Rect]) -> Option<Rect> {
    let gap = 6.0;
    let usable = Rect::from_ltrb(
        bounds.left() + gap,
        bounds.top() + gap,
        bounds.right() - gap,
        bounds.bottom() - gap,
    )?;
    let min_x = usable.left();
    let max_x = (usable.right() - size.0).max(min_x);
    let left = selection.left().floor();
    let top = selection.top().floor();
    let bottom = selection.bottom().ceil();
    for (x, y) in [
        (left, top - gap - size.1),
        (left + gap, top + gap),
        (left, bottom + gap),
    ] {
        let candidate = Rect::from_xywh(x.clamp(min_x, max_x), y, size.0, size.1)?;
        if candidate.left() >= usable.left()
            && candidate.top() >= usable.top()
            && candidate.right() <= usable.right()
            && candidate.bottom() <= usable.bottom()
            && !obstacles
                .iter()
                .any(|obstacle| candidate.intersect(obstacle).is_some())
        {
            return Some(candidate);
        }
    }
    None
}

/// 【选区尺寸】【比例弹层位置】按 anchor、size 和 bounds 先向下定位，放不下时向上翻转，返回四像素留白内的位置
pub fn popup(anchor: Rect, size: (f32, f32), bounds: Rect) -> Rect {
    let gap = 4.0;
    let x = anchor.left().round().clamp(
        bounds.left() + gap,
        (bounds.right() - size.0 - gap).max(bounds.left() + gap),
    );
    let mut y = anchor.bottom().round() + gap;
    if y + size.1 > bounds.bottom() - gap {
        y = anchor.top().round() - gap - size.1;
    }
    y = y.clamp(
        bounds.top() + gap,
        (bounds.bottom() - size.1 - gap).max(bounds.top() + gap),
    );
    Rect::from_xywh(x, y, size.0.max(1.0), size.1.max(1.0)).unwrap()
}
