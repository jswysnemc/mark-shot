//! 【列表控件】【滚动几何】历史与设置共用滚动条命中、端点和抓取偏移

use tiny_skia::Rect;

pub(crate) struct Geometry {
    pub track: Rect,
    pub handle: Rect,
    pub maximum: usize,
    pub page: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct Drag {
    offset: f32,
}

/// 【截图历史】【滚动几何】按 viewport、count、rows 和 scroll 返回原版窄滚动条，记录不足一屏时返回空值
pub(crate) fn geometry(
    viewport: Rect,
    count: usize,
    rows: usize,
    scroll: usize,
) -> Option<Geometry> {
    let maximum = count.checked_sub(rows).filter(|maximum| *maximum > 0)?;
    let track = rect(
        viewport.right() - 8.0,
        viewport.y() + 1.0,
        8.0,
        (viewport.height() - 2.0).max(1.0),
    );
    let height = (track.height() * rows as f32 / count as f32)
        .max(24.0)
        .min(track.height());
    let y = track.y() + (track.height() - height) * scroll.min(maximum) as f32 / maximum as f32;
    Some(Geometry {
        track,
        handle: rect(track.x() + 1.0, y, 6.0, height),
        maximum,
        page: rows,
    })
}

/// 【列表控件】【矩形构造】按位置和尺寸构造滚动条矩形，返回非空逻辑范围
fn rect(left: f32, top: f32, width: f32, height: f32) -> Rect {
    Rect::from_xywh(left, top, width.max(1.0), height.max(1.0)).unwrap()
}

impl Drag {
    /// 【截图历史】【滚动开始】记录 position 在 geometry 滑块内的抓取偏移，返回拖动状态
    pub fn new(position: (f64, f64), geometry: &Geometry) -> Self {
        Self {
            offset: position.1 as f32 - geometry.handle.y(),
        }
    }

    /// 【截图历史】【滚动移动】用 position 和 geometry 返回夹紧后的行偏移，无累计舍入误差
    pub fn value(self, position: (f64, f64), geometry: &Geometry) -> usize {
        let span = geometry.track.height() - geometry.handle.height();
        if span <= 0.0 {
            return 0;
        }
        let position = (position.1 as f32 - self.offset - geometry.track.y()).clamp(0.0, span);
        (position / span * geometry.maximum as f32).round() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【截图历史测试】【滚动边界】隐藏空滚动条，滑块两端到达精确行范围，拖回原点不漂移，无参数、无返回值
    #[test]
    fn scrollbar_drag_uses_the_grab_offset_and_exact_endpoints() {
        let viewport = rect(12.0, 82.0, 210.0, 392.0);
        assert!(geometry(viewport, 8, 8, 0).is_none());
        let bar = geometry(viewport, 30, 8, 0).unwrap();
        let point = (bar.handle.x() as f64 + 2.0, bar.handle.y() as f64 + 7.0);
        let drag = Drag::new(point, &bar);
        assert_eq!(drag.value(point, &bar), 0);
        assert_eq!(drag.value((point.0, -1000.0), &bar), 0);
        assert_eq!(drag.value((point.0, 1000.0), &bar), 22);
        assert_eq!(drag.value(point, &bar), 0);
    }
}
