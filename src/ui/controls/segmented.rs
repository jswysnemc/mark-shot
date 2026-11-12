//! 【界面控件】【分段选择】单选组共用连续底板与高亮选中段，替代并排大按钮

use tiny_skia::Rect;

use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
};

/// 【界面控件】【分段几何】按 bounds 与 count 返回第 index 段矩形，绘制与命中共用同一划分
///
/// 参数：bounds 整组边界、count 段数、index 段序号；返回该段在组内的矩形
pub fn segment(bounds: Rect, count: usize, index: usize) -> Rect {
    let count = count.max(1) as f32;
    let width = (bounds.width() - 4.0 - (count - 1.0) * 2.0) / count;
    Rect::from_xywh(
        bounds.left() + 2.0 + index as f32 * (width + 2.0),
        bounds.top() + 2.0,
        width.max(1.0),
        (bounds.height() - 4.0).max(1.0),
    )
    .unwrap()
}

/// 【界面控件】【分段绘制】用 painter 在 bounds 内按 labels 等分绘制单选组，无返回值
///
/// 参数：painter 目标画布、bounds 整组边界、labels 各段文字、
/// selected 当前选中段、hovered 指针停留段、enabled 是否可交互
pub fn segmented(
    painter: &mut Painter<'_>,
    bounds: Rect,
    labels: &[&str],
    selected: Option<usize>,
    hovered: Option<usize>,
    enabled: bool,
) {
    let palette = Palette::current();
    // 1. 【界面控件】【连续底板】整组先铺一层凹槽，各段不再单独成为按钮
    painter.fill_rounded(bounds, 6.0, palette.alternate);
    for (index, label) in labels.iter().enumerate() {
        let area = segment(bounds, labels.len(), index);
        let chosen = selected == Some(index);
        let hover = hovered == Some(index) && enabled && !chosen;
        // 2. 【界面控件】【选中高亮】选中段使用强调底色与细描边，禁用时只保留中性描边表示当前取值
        if chosen && enabled {
            super::frame(painter, area, palette.pressed, palette.accent, 4.0);
        } else if chosen {
            super::frame(painter, area, palette.alternate, palette.border, 4.0);
        } else if hover {
            painter.fill_rounded(area, 4.0, palette.base);
        }
        // 3. 【界面控件】【文字对比】文字保持正文颜色以保证小字号对比度，禁用整组使用次要颜色
        let style =
            TextStyle::new(11.0, if enabled { palette.text } else { palette.muted }).center();
        let text = super::fit_label(painter, label, 11.0, area.width() - 6.0);
        painter.text(
            area,
            &text,
            if chosen {
                style.weight(cosmic_text::Weight::SEMIBOLD)
            } else {
                style
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【分段控件测试】【等分几何】各段等宽等高从左到右排列，不重叠且落在 bounds 内
    #[test]
    fn segments_divide_the_bounds_without_overlap() {
        let bounds = Rect::from_xywh(10.0, 20.0, 300.0, 28.0).unwrap();
        for count in 1..=5 {
            let mut previous: Option<Rect> = None;
            for index in 0..count {
                let area = segment(bounds, count, index);
                assert!(area.left() >= bounds.left() && area.right() <= bounds.right());
                assert!(area.top() >= bounds.top() && area.bottom() <= bounds.bottom());
                if let Some(previous) = previous {
                    assert!(previous.right() < area.left());
                    assert!((previous.width() - area.width()).abs() < 0.01);
                }
                previous = Some(area);
            }
        }
    }
}
