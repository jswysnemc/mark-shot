//! 【悬停提示】【气泡】工具栏按钮悬停 TOOLTIP_DELAY 后显示名称与快捷键，移开立即隐藏

use std::time::{Duration, Instant};

use tiny_skia::Rect;

use crate::renderer::Painter;
use crate::renderer::text::TextStyle;
use crate::theme::font;

use super::{Orientation, Toolbar, ToolbarItem};

/// 【悬停提示】【延迟】悬停持续该时长才显示气泡，防止扫过工具栏时闪烁
pub const TOOLTIP_DELAY: Duration = Duration::from_millis(500);

const PAD_X: f32 = 10.0;
const HEIGHT: f32 = 24.0;
/// gap between the hovered button and the bubble
const GAP: f32 = 6.0;
/// distance kept from the monitor edges
const EDGE: f32 = 4.0;
const CORNER: f32 = 6.0;

/// 【悬停提示】【可见气泡】当前应绘制的气泡；归属、下标、位置与文本在显示时冻结
pub struct Tooltip {
    /// false = 主工具栏，true = 动作工具栏
    pub owner_actions: bool,
    pub index: usize,
    pub monitor_idx: usize,
    /// monitor-local bubble rect
    pub rect: Rect,
    pub text: String,
}

/// 【悬停提示】【文案】按 chinese 返回按钮显示名，已绑定快捷键时追加 (key)
pub fn text(item: ToolbarItem, chinese: bool) -> Option<String> {
    let entry = super::catalog::entry(item)?;
    let label = entry.label(chinese);
    let key = entry
        .button
        .action()
        .and_then(|action| crate::keys::map().label(action));
    Some(match key {
        Some(key) if !key.is_empty() => format!("{label} ({key})"),
        _ => label.to_string(),
    })
}

/// 【悬停提示】【到期时刻】返回气泡应显示的时刻；已显示、不可显示或未悬停时返回空值
pub fn deadline(
    current: Option<&Tooltip>,
    owner_actions: bool,
    toolbar: &Toolbar,
) -> Option<Instant> {
    if current.is_some_and(|tip| tip.owner_actions == owner_actions) {
        return None;
    }
    let index = toolbar.hovered?;
    if toolbar.interferes
        || toolbar.window_bar
        || toolbar.disabled.contains(&index)
        || !matches!(toolbar.items.get(index), Some(ToolbarItem::Button(_)))
    {
        return None;
    }
    let at = toolbar.hover_started? + TOOLTIP_DELAY;
    // a past deadline already had its wake-up; repeating it would spin the loop
    (at > Instant::now()).then_some(at)
}

/// 【悬停提示】【气泡定位】按悬停单元格返回气泡矩形，放在工具栏朝向屏幕中心的一侧并收敛边界
pub fn bubble_rect(
    toolbar: &Toolbar,
    index: usize,
    text_w: f32,
    bounds: (f32, f32),
) -> Option<Rect> {
    let cell = toolbar.cell(index)?;
    let (x, y) = toolbar.render_pos;
    let cell = Rect::from_xywh(cell.left() + x, cell.top() + y, cell.width(), cell.height())?;
    let w = text_w + PAD_X * 2.0;
    let h = HEIGHT;
    let (x, y) = match toolbar.orientation {
        Orientation::Horizontal => {
            let x = cell.left() + cell.width() / 2.0 - w / 2.0;
            let above = cell.top() + cell.height() / 2.0 > bounds.1 / 2.0;
            let y = if above {
                cell.top() - GAP - h
            } else {
                cell.bottom() + GAP
            };
            (x, y)
        }
        Orientation::Vertical => {
            let y = cell.top() + cell.height() / 2.0 - h / 2.0;
            let left = cell.left() + cell.width() / 2.0 > bounds.0 / 2.0;
            let x = if left {
                cell.left() - GAP - w
            } else {
                cell.right() + GAP
            };
            (x, y)
        }
    };
    Rect::from_xywh(
        x.clamp(EDGE, (bounds.0 - w - EDGE).max(EDGE)),
        y.clamp(EDGE, (bounds.1 - h - EDGE).max(EDGE)),
        w,
        h,
    )
}

/// 【悬停提示】【气泡绘制】按 clip 相交把气泡绘入 p；不属于本显示器时跳过
pub fn draw_tooltip(
    p: &mut Painter,
    tip: Option<&Tooltip>,
    monitor_idx: usize,
    clip: Option<&Rect>,
) {
    let Some(tip) = tip else { return };
    if tip.monitor_idx != monitor_idx {
        return;
    }
    if let Some(clip) = clip
        && !crate::utils::rects_overlap(clip, &tip.rect)
    {
        return;
    }
    let cfg = &crate::config::get().toasts;
    p.fill_rounded(tip.rect, CORNER, cfg.background.get());
    p.panel_border(
        tip.rect.left(),
        tip.rect.top(),
        tip.rect.width(),
        tip.rect.height(),
        CORNER,
        1.0,
    );
    p.text(
        tip.rect,
        &tip.text,
        TextStyle::new(font::label(), cfg.text.get()).center(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;
    use crate::ui::toolbar::ToolbarButton;

    /// 【悬停提示】【到期判定】无参数，仅有效按钮的悬停给出时刻，隐藏、禁用与已显示返回空值
    #[test]
    fn deadline_only_for_enabled_hovered_buttons() {
        let mut toolbar = Toolbar::new();
        assert!(deadline(None, false, &toolbar).is_none());
        toolbar.hovered = Some(0);
        toolbar.hover_started = Some(Instant::now());
        let at = deadline(None, false, &toolbar).unwrap();
        assert!(at > Instant::now() && at <= Instant::now() + TOOLTIP_DELAY);

        toolbar.disabled.push(0);
        assert!(deadline(None, false, &toolbar).is_none());
        toolbar.disabled.clear();

        toolbar.hovered = Some(2);
        assert!(deadline(None, false, &toolbar).is_none());
        toolbar.hovered = Some(0);

        toolbar.interferes = true;
        assert!(deadline(None, false, &toolbar).is_none());
        toolbar.interferes = false;

        let tip = Tooltip {
            owner_actions: false,
            index: 0,
            monitor_idx: 0,
            rect: Rect::from_xywh(0.0, 0.0, 10.0, 10.0).unwrap(),
            text: String::new(),
        };
        assert!(deadline(Some(&tip), false, &toolbar).is_none());
        assert!(deadline(Some(&tip), true, &toolbar).is_some());
    }

    /// 【悬停提示】【文案判定】无参数，分隔线返回空值，按钮返回目录名前缀
    #[test]
    fn text_skips_separator_and_starts_with_label() {
        assert!(text(ToolbarItem::Seperator, true).is_none());
        let item = ToolbarItem::Button(ToolbarButton::Tool(Tool::Rectangle));
        let label = crate::ui::toolbar::catalog::entry(item)
            .unwrap()
            .label(true);
        assert!(text(item, true).unwrap().starts_with(label));
    }

    /// 【悬停提示】【边界收敛】无参数，每个单元格的气泡都在显示器范围内
    #[test]
    fn bubble_stays_inside_the_monitor() {
        let toolbar = Toolbar::new();
        let bounds = (640.0, 480.0);
        for index in 0..toolbar.items.len() {
            let Some(rect) = bubble_rect(&toolbar, index, 80.0, bounds) else {
                continue;
            };
            assert!(rect.left() >= 0.0 && rect.right() <= bounds.0);
            assert!(rect.top() >= 0.0 && rect.bottom() <= bounds.1);
        }
    }
}
