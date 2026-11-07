//! 【滚动条输入测试】【鼠标事务】对照原版轨道、滑块、中键和按住重复，不接入真实桌面

use super::*;
use crate::features::ocr_result::{layout::Layout, table::Table};
use cosmic_text::FontSystem;

/// 【滚动条输入测试】【固定窗口】用 kind、width 创建原版采集对应的真实数据，返回视区、字体和表格矩形，不使用期望列宽代替测量
fn view(kind: &str, width: u32) -> (TableView, FontSystem, Rect) {
    let rows = if kind == "wide" { 3 } else { 40 };
    let columns = if kind == "tall" { 3 } else { 8 };
    let table = Table {
        rows: (0..rows)
            .map(|row| {
                (0..columns)
                    .map(|column| {
                        if kind == "tall" {
                            (row * columns + column).to_string()
                        } else if kind == "oversized" && column == 0 {
                            "W".repeat(180)
                        } else {
                            format!("A long column value {}", row * columns + column)
                        }
                    })
                    .collect()
            })
            .collect(),
    };
    let mut fonts = crate::fonts::ocr_ui::new();
    (
        TableView::new(table, &mut fonts),
        fonts,
        Layout::new((width, 460), true).table_body,
    )
}

/// 【滚动条输入测试】【区域中心】按 body、rect 返回窗口逻辑位置，避免本地与场景坐标混淆
fn center(body: Rect, rect: Rect) -> (f64, f64) {
    (
        f64::from(body.x() + rect.x() + rect.width() / 2.0),
        f64::from(body.y() + rect.y() + rect.height() / 2.0),
    )
}

/// 【滚动条输入测试】【原版鼠标状态】对照两种主题、三种宽度和四种溢出数据的实际轨道、箭头、滑块、中键与右键状态，无参数、无返回值
#[test]
fn actual_qt_pointer_states_match_native_scrollbar_transactions() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-ocr-table-scroll.json"
    ))
    .unwrap();
    let now = Instant::now();
    for case in fixture["cases"].as_array().unwrap() {
        let (mut view, _, body) = view(
            case["kind"].as_str().unwrap(),
            case["width"].as_u64().unwrap() as u32,
        );
        for (axis, name) in [(0, "horizontal"), (1, "vertical")] {
            if view.scroll_bars(body)[axis].is_none() {
                continue;
            }
            for (action, part, button) in [
                ("add-line", Part::NextLine, LEFT_BUTTON),
                ("add-page", Part::NextPage, LEFT_BUTTON),
                ("middle-page", Part::NextPage, 0x112),
                ("right-down", Part::Slider, RIGHT_BUTTON),
                ("drag-max", Part::Slider, LEFT_BUTTON),
            ] {
                let expected = case["states"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|state| state["name"] == format!("{name}-{action}"));
                let Some(expected) = expected else {
                    continue;
                };
                view.scroll = (0.0, 0.0);
                view.bars.cancel();
                let bar = view.scroll_bars(body)[axis].take().unwrap();
                let rect = match part {
                    Part::NextLine => bar.next_line.unwrap(),
                    Part::NextPage => bar.next_page.unwrap(),
                    _ => bar.slider,
                };
                let position = center(body, rect);
                assert!(view.bar_press(body, position, button, now).is_some());
                if action == "drag-max" {
                    let mut end = center(body, bar.groove);
                    match bar.axis {
                        Axis::Horizontal => end.0 = f64::from(body.x() + bar.groove.right() - 1.0),
                        Axis::Vertical => end.1 = f64::from(body.y() + bar.groove.bottom() - 1.0),
                    }
                    view.bar_motion(body, end);
                }
                assert_eq!(
                    serde_json::json!([view.scroll.0, view.scroll.1]),
                    serde_json::json!([
                        expected["state"]["offset"][0].as_f64().unwrap(),
                        expected["state"]["offset"][1].as_f64().unwrap()
                    ]),
                    "{} {} {name}-{action}",
                    case["kind"],
                    case["width"]
                );
                view.bars.release(button);
                assert!(view.bars.deadline().is_none());
            }
        }
    }
}

/// 【滚动条输入测试】【原版重复终止】初始五百毫秒不重复，箭头每五十毫秒继续一项，轨道到点击目标后停止，无参数、无返回值
#[test]
fn held_controls_repeat_only_until_release_or_the_page_target() {
    let (mut view, _, body) = view("both", 760);
    let now = Instant::now();
    let bar = view.scroll_bars(body)[0].take().unwrap();
    view.bar_press(body, center(body, bar.next_line.unwrap()), LEFT_BUTTON, now);
    assert!(!view.bar_wake(body, now + Duration::from_millis(499)));
    assert_eq!(view.scroll_ranges(body)[0].index(view.scroll.0), 1);
    assert!(view.bar_wake(body, now + Duration::from_millis(500)));
    assert!(view.bar_wake(body, now + Duration::from_millis(550)));
    assert_eq!(view.scroll_ranges(body)[0].index(view.scroll.0), 3);
    assert!(!view.bars.release(RIGHT_BUTTON));
    assert!(view.bars.release(LEFT_BUTTON));
    assert!(!view.bar_wake(body, now + Duration::from_secs(2)));
    view.scroll = (0.0, 0.0);
    view.bars.cancel();
    let bar = view.scroll_bars(body)[1].take().unwrap();
    view.bar_press(body, center(body, bar.next_page.unwrap()), LEFT_BUTTON, now);
    assert_eq!(view.scroll_ranges(body)[1].index(view.scroll.1), 11);
    assert!(view.bar_wake(body, now + Duration::from_millis(500)));
    assert_eq!(view.scroll_ranges(body)[1].index(view.scroll.1), 20);
    assert!(view.bars.deadline().is_none());
    assert!(!view.bar_wake(body, now + Duration::from_secs(1)));
}

/// 【滚动条输入测试】【拖动与编辑隔离】右键不抢占左键拖动，滑块按下不提交单元格草稿，取消清理全部期限，无参数、无返回值
#[test]
fn scrollbar_drag_does_not_select_cells_or_commit_editor_drafts() {
    let (mut view, mut fonts, body) = view("both", 760);
    let now = Instant::now();
    view.active = Some((0, 0));
    view.begin_edit(&mut fonts);
    view.editor.as_mut().unwrap().insert(&mut fonts, "draft");
    let original = view.table.rows[0][0].clone();
    let bar = view.scroll_bars(body)[0].take().unwrap();
    let position = center(body, bar.slider);
    view.bar_press(body, position, LEFT_BUTTON, now);
    view.bar_press(body, position, RIGHT_BUTTON, now);
    assert!(!view.bars.release(RIGHT_BUTTON));
    assert!(view.bar_motion(body, (10000.0, position.1)));
    assert_eq!(view.active, Some((0, 0)));
    assert_eq!(view.table.rows[0][0], original);
    assert!(view.editor.is_some());
    assert!(view.bars.cancel());
    assert!(view.bars.deadline().is_none());
    let offset = view.scroll;
    view.bar_motion(body, position);
    assert_eq!(view.scroll, offset);
}
