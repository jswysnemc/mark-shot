//! 【表格网格测试】【原版记录】验证完整窗口中的表头、内容列宽和稳定编辑尺寸

use super::*;
use crate::features::ocr_result::{layout::Layout, table_view::TableView};

/// 【表格网格测试】【数据重建】按 kind 返回与原版探针对应的真实单元格，不读取用户配置或模型
fn table(kind: &str) -> Table {
    let rows = if kind == "tall" { 40 } else { 3 };
    let columns = if kind == "wide" { 8 } else { 3 };
    Table {
        rows: (0..rows)
            .map(|row| {
                (0..columns)
                    .map(|column| {
                        if kind == "content" {
                            [
                                "Item", "Count", "Total", "Paper", "12", "36", "Ink", "2", "24",
                            ][row * columns + column]
                                .to_owned()
                        } else if kind == "multiline" && row == 1 && column == 0 {
                            "first\nsecond\nthird".into()
                        } else if kind == "wide" {
                            format!("A long column value {}", row * columns + column)
                        } else {
                            (row * columns + column).to_string()
                        }
                    })
                    .collect()
            })
            .collect(),
    }
}

/// 【表格网格测试】【矩形比较】将 actual 与 expected 的四项逻辑坐标逐项比较，label 用于标识失败位置，无返回值
fn rectangle(actual: Rect, expected: &serde_json::Value, label: &str) {
    let values = [actual.x(), actual.y(), actual.width(), actual.height()];
    for (index, value) in values.iter().enumerate() {
        assert_eq!(
            *value,
            expected[index].as_f64().unwrap() as f32,
            "{label} component {index}"
        );
    }
}

/// 【表格网格测试】【真实窗口几何】对照两种主题、三种宽度和五种内容的原版初始网格，不用截图推测几何，无参数、无返回值
#[test]
fn grid_matches_actual_qt_headers_viewport_and_content_columns() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-ocr-table-grid.json"
    ))
    .unwrap();
    let mut mismatches = Vec::new();
    for case in reference["cases"].as_array().unwrap() {
        let mut fonts = crate::fonts::ocr_ui::new();
        let mut view = TableView::new(table(case["kind"].as_str().unwrap()), &mut fonts);
        let layout = Layout::new((case["width"].as_u64().unwrap() as u32, 460), true);
        let expected = &case["states"][0]["state"];
        rectangle(layout.table_body, &expected["table"], "table body");
        let grid = view.grid(layout.table_body);
        rectangle(grid.viewport, &expected["viewport"], "viewport");
        rectangle(
            grid.horizontal_header,
            &expected["horizontal_header"],
            "horizontal header",
        );
        rectangle(
            grid.vertical_header,
            &expected["vertical_header"],
            "vertical header",
        );
        for (column, expected) in expected["columns"].as_array().unwrap().iter().enumerate() {
            if grid.column(column).1 != expected["width"].as_f64().unwrap() as f32 {
                mismatches.push(format!(
                    "{} {} column {column}: native {} / Qt {}",
                    case["kind"],
                    case["width"],
                    grid.column(column).1,
                    expected["width"]
                ));
            }
        }
        for (name, actual) in [
            ("horizontal_scroll", grid.horizontal_scroll),
            ("vertical_scroll", grid.vertical_scroll),
        ] {
            assert_eq!(
                actual.is_some(),
                expected[name]["visible"].as_bool().unwrap()
            );
            if let Some(actual) = actual {
                rectangle(actual, &expected[name]["rect"], name);
            }
        }
        // 1. 【表格网格测试】【编辑稳定】编辑长多行草稿并确认后，原版不重新分配列宽或增加行高
        view.active = Some((1, 0));
        view.begin_edit(&mut fonts);
        view.editor
            .as_mut()
            .unwrap()
            .insert(&mut fonts, "Changed value\nsecond line");
        view.finish(true);
        let after = view.grid(layout.table_body);
        for column in 0..view.table.columns() {
            assert_eq!(grid.column(column), after.column(column));
        }
        assert_eq!(after.content_size.1, grid.content_size.1);
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// 【表格网格测试】【命中隔离】验证单元格左闭右开边界、表头与空白不命中、两方向滚动和缩小恢复，无参数、无返回值
#[test]
fn hit_testing_excludes_headers_empty_space_and_scrollbars() {
    let mut fonts = crate::fonts::ocr_ui::new();
    let mut view = TableView::new(table("wide"), &mut fonts);
    let body = Layout::new((380, 460), true).table_body;
    let grid = view.grid(body);
    let point = |x, y| (f64::from(body.x() + x), f64::from(body.y() + y));
    assert_eq!(
        view.cell_at(body, point(grid.viewport.x(), grid.viewport.y())),
        Some((0, 0))
    );
    assert_eq!(
        view.cell_at(body, point(grid.viewport.x() - 0.1, grid.viewport.y())),
        None
    );
    assert_eq!(
        view.cell_at(body, point(grid.viewport.x(), grid.viewport.y() - 0.1)),
        None
    );
    assert_eq!(
        view.cell_at(
            body,
            point(grid.viewport.x(), grid.viewport.y() + 3.0 * ROW_HEIGHT)
        ),
        None
    );
    let bar = grid.horizontal_scroll.unwrap();
    assert_eq!(
        view.cell_at(body, point(bar.x() + 20.0, bar.y() + 2.0)),
        None
    );
    // 1. 【表格网格测试】【原版逐项】单个滚轮事件最多移动一页，连续事件到达末端后仍不能越界
    for _ in 0..10 {
        view.wheel(
            body,
            crate::features::ocr_result::table_scroll_model::legacy(
                (10000.0, 10000.0),
                crate::keys::Mods::default(),
            ),
        );
    }
    assert_eq!(view.scroll, grid.clamp_scroll((10000.0, 10000.0)));
    view.active = Some((0, 0));
    view.ensure_visible(body);
    assert_eq!(view.scroll, (0.0, 0.0));
}

/// 【表格网格测试】【超宽单元格】活动单元格宽于视区时保持非负滚动，空表格也能安全生成布局，无参数、无返回值
#[test]
fn oversized_and_empty_grids_keep_finite_nonnegative_offsets() {
    let columns = Columns {
        widths: vec![1000.0, 40.0],
        gutter: 29.0,
    };
    let grid = columns.grid((364.0, 100.0), 40);
    assert!(grid.horizontal_scroll.is_some() && grid.vertical_scroll.is_some());
    assert_eq!(grid.clamp_scroll((-20.0, -10.0)), (0.0, 0.0));
    let empty = Columns {
        widths: vec![],
        gutter: 29.0,
    }
    .grid((364.0, 100.0), 0);
    assert_eq!(empty.visible_columns(0.0), 0..0);
    assert_eq!(empty.column_at(0.0), None);
    assert_eq!(empty.clamp_scroll((30.0, 30.0)), (0.0, 0.0));
}

/// 【表格网格测试】【选择与指针】单击只选择单元格，开启编辑才显示文字指针，滚动后表头不会取得编辑指针，无参数、无返回值
#[test]
fn single_click_selects_without_editing_and_headers_keep_default_cursor() {
    use crate::backend::window::{WindowContent, WindowCursor};
    use crate::features::ocr_result::{
        document::OcrDocument,
        window::{ResultWindow, View},
    };
    let mut state = ResultWindow::new(OcrDocument {
        text: "base".into(),
        lines: vec![],
        png: crate::utils::encode_png(&tiny_skia::Pixmap::new(2, 2).unwrap()),
    })
    .unwrap();
    state.table = Some(TableView::new(table("content"), &mut state.fonts));
    state.view = View::Table;
    state.refresh_layout();
    let body = state.layout.table_body;
    let grid = state.table.as_ref().unwrap().grid(body);
    let point = (
        f64::from(body.x() + grid.viewport.x() + 10.0),
        f64::from(body.y() + grid.viewport.y() + 10.0),
    );
    state
        .table
        .as_mut()
        .unwrap()
        .pointer(&mut state.fonts, body, point, false, 1);
    assert_eq!(state.table.as_ref().unwrap().active, Some((0, 0)));
    assert!(state.table.as_ref().unwrap().editor.is_none());
    assert!(matches!(state.cursor(point), WindowCursor::Default));
    state.table.as_mut().unwrap().begin_edit(&mut state.fonts);
    assert!(matches!(state.cursor(point), WindowCursor::Text));
    state.table.as_mut().unwrap().scroll.1 = 20.0;
    let header_point = (point.0, f64::from(body.y() + grid.viewport.y() - 10.0));
    assert!(matches!(state.cursor(header_point), WindowCursor::Default));
}

/// 【表格滚动测试】【真实刻度】原版普通滚轮每刻度移动三行，而不是直接使用协议像素值，无参数、无返回值
#[test]
fn table_wheel_step_matches_the_actual_qt_three_row_movement() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-ocr-table-scroll.json"
    ))
    .unwrap();
    let case = reference["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["theme"] == "dark" && case["width"] == 760 && case["kind"] == "tall")
        .unwrap();
    let expected = case["states"]
        .as_array()
        .unwrap()
        .iter()
        .find(|state| state["name"] == "wheel-down")
        .unwrap();
    let mut fonts = crate::fonts::ocr_ui::new();
    let mut view = TableView::new(table("tall"), &mut fonts);
    view.wheel(
        Layout::new((760, 460), true).table_body,
        crate::backend::window::ScrollInput {
            angle: (0.0, -120.0),
            ..crate::backend::window::ScrollInput::legacy((0.0, 0.0), (0.0, 15.0))
        },
    );
    assert_eq!(
        view.scroll.1,
        expected["state"]["offset"][1].as_f64().unwrap() as f32
    );
}
