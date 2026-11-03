use crate::scroll::panel;
use tiny_skia::Rect;

fn region(width: f32, height: f32) -> Rect {
    Rect::from_xywh(0.0, 0.0, width, height).unwrap()
}

#[test]
fn panel_sits_right_of_the_region_when_there_is_room() {
    let layout = panel::layout(region(400.0, 300.0), (1920.0, 1080.0), None);
    assert!(layout.rect.left() >= 400.0);
    assert!(layout.rect.right() <= 1920.0);
}

#[test]
fn panel_flips_to_the_left_when_the_right_edge_is_used_up() {
    // flips left when right space is less than panel width
    let layout = panel::layout(region(1900.0, 300.0), (1920.0, 1080.0), None);
    assert!(layout.rect.right() <= 1900.0);
    assert!(layout.rect.left() >= 0.0);
}

#[test]
fn full_screen_region_keeps_the_panel_inside_the_monitor() {
    let layout = panel::layout(region(1920.0, 1080.0), (1920.0, 1080.0), None);
    assert!(layout.rect.left() >= 0.0);
    assert!(layout.rect.right() <= 1920.0);
    assert!(layout.rect.top() >= 0.0);
    assert!(layout.rect.bottom() <= 1080.0);
}

#[test]
fn preview_reserves_vertical_space_and_buttons_stay_inside() {
    let with_preview = panel::layout(region(400.0, 300.0), (1920.0, 1080.0), Some((132, 400)));
    let without = panel::layout(region(400.0, 300.0), (1920.0, 1080.0), None);
    assert!(with_preview.rect.height() > without.rect.height());
    for (_, rect) in &with_preview.buttons {
        assert!(rect.right() <= with_preview.rect.right());
        assert!(rect.bottom() <= with_preview.rect.bottom());
    }
}

#[test]
fn status_and_length_use_separate_rows() {
    let layout = panel::layout(region(400.0, 300.0), (1920.0, 1080.0), Some((132, 400)));
    assert!(
        layout.length.top() >= layout.status.bottom(),
        "length row {:?} overlaps status row {:?}",
        layout.length,
        layout.status
    );
    assert!(
        layout
            .buttons
            .iter()
            .all(|(_, r)| r.top() >= layout.length.bottom())
    );
    // both rows stay within panel bounds
    assert!(layout.status.right() <= layout.rect.right());
    assert!(layout.length.right() <= layout.rect.right());
}

#[test]
fn hit_test_finds_buttons_and_nothing_outside() {
    let layout = panel::layout(region(400.0, 300.0), (1920.0, 1080.0), None);
    let (button, rect) = layout.buttons.first().expect("panel has buttons");
    let center = (
        (rect.left() + rect.width() / 2.0) as f64,
        (rect.top() + rect.height() / 2.0) as f64,
    );
    assert_eq!(panel::hit_test(&layout, center), Some(*button));
    assert_eq!(panel::hit_test(&layout, (0.0, 0.0)), None);
}

#[test]
fn toggle_axis_icon_switches_between_vertical_and_horizontal() {
    use crate::scroll::session::ScrollSession;
    use crate::ui::icons;

    let mut session = ScrollSession::new(region(40.0, 160.0), 0).unwrap();
    assert_eq!(
        panel::button_icon(panel::PanelButton::ToggleAxis, &session),
        icons::SCROLL
    );

    assert!(session.toggle_axis());
    assert_eq!(
        panel::button_icon(panel::PanelButton::ToggleAxis, &session),
        icons::SCROLL_HORIZONTAL
    );
}

#[test]
fn toggle_axis_availability_before_and_after_growth() {
    use crate::scroll::session::ScrollSession;

    let session = ScrollSession::new(region(40.0, 160.0), 0).unwrap();
    assert!(panel::is_button_enabled(
        panel::PanelButton::ToggleAxis,
        &session
    ));
    assert!(panel::is_button_enabled(
        panel::PanelButton::TogglePause,
        &session
    ));
    assert!(panel::is_button_enabled(
        panel::PanelButton::Finish,
        &session
    ));
    assert!(panel::is_button_enabled(
        panel::PanelButton::Cancel,
        &session
    ));
}
