use mark_shot::keys::Key;
use mark_shot::types::MarkerShape;
use mark_shot::ui::choice_popover::{ChoiceEvent, ChoiceOption, ChoicePopover};
use mark_shot::ui::panel::{AnimatedPanel, UiPanel};

/// 【选择测试】【面板构造】返回包含全部标记的已显示面板，固定显示器本地偏移
fn panel() -> ChoicePopover {
    let mut panel = ChoicePopover::new(
        "Marker shapes",
        MarkerShape::ALL
            .map(|shape| ChoiceOption {
                label: shape.label(),
            })
            .to_vec(),
    );
    panel.open = true;
    panel.opacity = 1.0;
    panel.render_pos = (75.0, 20.0);
    panel.active = Some(0);
    panel
}

/// 【选择测试】【全部命中】各单元中心返回正确索引，标题和单元间距不触发选择
#[test]
fn grid_hit_testing_uses_visible_cells_and_skips_padding() {
    let panel = panel();
    for index in 0..24 {
        let rect = panel.layout.item_rect(index, 0).unwrap();
        assert_eq!(
            panel.item_at((
                (75.0 + rect.left() + rect.width() / 2.0) as f64,
                (20.0 + rect.top() + rect.height() / 2.0) as f64
            )),
            Some(index)
        );
    }
    assert_eq!(panel.item_at((85.0, 30.0)), None);
    let first = panel.layout.item_rect(0, 0).unwrap();
    assert_eq!(
        panel.item_at((
            (75.0 + first.right() + panel.layout.gap / 2.0) as f64,
            (20.0 + first.top() + 10.0) as f64
        )),
        None
    );
}

/// 【选择测试】【窄屏布局】缩小宽度减少列数，缩小高度只显示可见行，所有条目仍可滚动访问
#[test]
fn narrow_and_short_layouts_remain_inside_the_available_space() {
    let mut panel = panel();
    assert!(panel.resize((180.0, 190.0)));
    assert!(panel.layout.size.0 <= 180.0);
    assert!(panel.layout.size.1 <= 190.0);
    assert!(panel.layout.columns < 6);
    assert!(panel.layout.visible_rows < panel.layout.rows);
    assert!(panel.scroll_rows(1));
    assert!(panel.layout.item_rect(0, panel.first_row).is_none());
    panel.ensure_visible(23);
    assert!(panel.layout.item_rect(23, panel.first_row).is_some());
    panel.ensure_visible(0);
    assert_eq!(panel.first_row, 0);
}

/// 【选择测试】【键盘历史隔离】方向、首尾与 Tab 只移动预选；Enter 返回选择，不改变业务默认值
#[test]
fn keyboard_navigation_scrolls_without_committing_the_active_option() {
    let mut panel = panel();
    panel.resize((180.0, 190.0));
    assert_eq!(panel.key(Key::End, false), Some(ChoiceEvent::Changed));
    assert_eq!(panel.hovered, Some(23));
    assert_eq!(panel.active, Some(0));
    assert!(panel.layout.item_rect(23, panel.first_row).is_some());
    assert_eq!(panel.key(Key::Home, false), Some(ChoiceEvent::Changed));
    assert_eq!(panel.first_row, 0);
    assert_eq!(panel.key(Key::Tab, true), Some(ChoiceEvent::Changed));
    assert_eq!(panel.hovered, Some(23));
    assert_eq!(
        panel.key(Key::Enter, false),
        Some(ChoiceEvent::Selected(23))
    );
    assert_eq!(panel.key(Key::Escape, false), Some(ChoiceEvent::Close));
    assert_eq!(panel.key(Key::Char('k'), false), None);
}

/// 【选择测试】【滚动边界】大幅正负滚动不会越界，也不修改已确认的条目
#[test]
fn scrolling_clamps_to_the_first_and_last_page() {
    let mut panel = panel();
    panel.resize((180.0, 190.0));
    assert!(panel.scroll_rows(i32::MAX));
    assert_eq!(
        panel.first_row,
        panel.layout.rows - panel.layout.visible_rows
    );
    assert!(!panel.scroll_rows(1));
    assert!(panel.scroll_rows(i32::MIN));
    assert_eq!(panel.first_row, 0);
    assert_eq!(panel.active, Some(0));
}

/// 【选择测试】【动画结束】打开和关闭动画到达目标后停止轮询，关闭面板不再命中
#[test]
fn a_closed_popover_has_no_visible_rectangle_or_animation() {
    let mut panel = panel();
    panel.opacity = 0.0;
    assert!(panel.is_animating());
    panel.animate_step(1.0);
    assert!(!panel.is_animating());
    assert!(panel.rect().is_some());
    panel.open = false;
    panel.animate_step(1.0);
    assert!(!panel.is_animating());
    assert!(panel.rect().is_none());
    assert_eq!(panel.item_at((90.0, 90.0)), None);
}

/// 【选择测试】【空条目】空面板和越界单元不会导致除零或生成选择事件
#[test]
fn empty_popovers_do_not_produce_a_selection() {
    let mut panel = ChoicePopover::new("Empty", Vec::new());
    panel.open = true;
    panel.opacity = 1.0;
    assert_eq!(panel.key(Key::Enter, false), None);
    assert_eq!(panel.item_at((20.0, 60.0)), None);
    assert!(!panel.scroll_rows(1));
    assert!(panel.layout.item_rect(0, 0).is_none());
}

/// 【选择测试】【滚动取消】小数滚轮累计后取消，不把剩余步数带入下一次交互
#[test]
fn canceling_scroll_discards_fractional_rows() {
    let mut panel = panel();
    panel.resize((180.0, 190.0));
    assert!(!panel.scroll(0.6));
    panel.cancel_scroll();
    assert!(!panel.scroll(0.6));
    assert_eq!(panel.first_row, 0);
}
