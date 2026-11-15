//! 【图片窗口测试】【尺寸隔离】调整窗口不能改变原图、标注几何和导出内容

use super::*;

/// 【图片窗口测试】【单行布局】启用图片窗口后工具栏与窗口按钮共用一行，图片视口只为合并栏留白
#[test]
fn image_window_reserves_chrome_and_keeps_all_tools_on_one_row() {
    let mut state = crate::app::input::tests::editor();
    let minimum = layout::minimum();
    let size = (minimum.0.max(900), minimum.1.max(600));
    state.file_canvas = FileCanvas::new(
        tiny_skia::Pixmap::new(160, 100).unwrap(),
        layout::viewport(size),
    );
    state.image_window.enabled = true;
    state.fullscreen_scope = true;
    state.selection.zone = Some(state.file_canvas.as_ref().unwrap().viewport);
    resize(&mut state, size, &mut 0);
    let cells = &state.toolbar.cells;
    assert!(cells.iter().all(|cell| cell.top() == cells[0].top()));
    assert!(state.toolbar.size.0 + 16.0 <= size.0 as f32);
    assert!(state.toolbar.window_bar);
    assert_eq!(state.toolbar.position, layout::toolbar_position());
    let bar = crate::ui::image_window::layout::bar_height(state.toolbar.size.1);
    let minimize = crate::ui::image_window::layout::button(size, bar, 2);
    assert!(state.toolbar.position.0 + state.toolbar.size.0 < minimize.left());
    assert_eq!(minimize.top() + minimize.height() * 0.5, bar * 0.5);
    let viewport = state.navigation.window.unwrap();
    assert_eq!(viewport.top(), bar);
    assert!(
        !state
            .toolbar
            .items
            .contains(&crate::ui::toolbar::ToolbarItem::Button(
                crate::ui::toolbar::ToolbarButton::Cancel
            ))
    );
    assert_eq!(minimize.height(), state.toolbar.button_size());
    crate::ui::image_window::draw(&mut state, 0);
    let edge = (state.toolbar.position.0 + state.toolbar.size.0 - 1.0).round() as u32;
    assert_eq!(
        state.canvas[0].pixel(edge, (bar * 0.5) as u32),
        state.canvas[0].pixel(edge + 6, (bar * 0.5) as u32),
        "embedded toolbar must not retain a separate panel border"
    );
}

/// 【图片窗口测试】【工具输入】顶部工具按钮仍切换绘图工具，右侧窗口按钮独立执行，空白握柄只请求窗口移动
#[test]
fn merged_bar_routes_tools_window_controls_and_drag_separately() {
    use crate::{
        types::MouseButton,
        ui::{
            image_window::{Hit, layout as bar},
            toolbar::{ToolbarButton, ToolbarItem},
        },
    };
    let mut state = crate::app::input::tests::editor();
    let minimum = layout::minimum();
    let size = (minimum.0.max(1000), 600);
    state.file_canvas = FileCanvas::new(
        tiny_skia::Pixmap::new(160, 100).unwrap(),
        layout::viewport(size),
    );
    state.image_window.enabled = true;
    state.fullscreen_scope = true;
    state.selection.zone = Some(state.file_canvas.as_ref().unwrap().viewport);
    resize(&mut state, size, &mut 0);
    let index = state
        .toolbar
        .items
        .iter()
        .position(|item| {
            matches!(
                item,
                ToolbarItem::Button(ToolbarButton::Tool(crate::tools::Tool::Rectangle))
            )
        })
        .unwrap();
    let cell = state.toolbar.cell(index).unwrap();
    let point = (
        state.toolbar.render_pos.0 + cell.left() + cell.width() * 0.5,
        state.toolbar.render_pos.1 + cell.top() + cell.height() * 0.5,
    );
    crate::app::input::handle_pointer_move(
        &mut state,
        0,
        f64::from(point.0),
        f64::from(point.1),
        &mut 0,
    );
    crate::app::input::handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    crate::app::input::handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    assert_eq!(state.selected_tool, crate::tools::Tool::Rectangle);
    assert!(state.image_window.action.is_none());
    let height = bar::bar_height(state.toolbar.size.1);
    for (index, hit, action) in [
        (
            1,
            Hit::Maximize,
            crate::types::image_window::Action::ToggleMaximize,
        ),
        (
            2,
            Hit::Minimize,
            crate::types::image_window::Action::Minimize,
        ),
    ] {
        let button = bar::button(size, height, index);
        let point = (
            f64::from(button.left() + button.width() * 0.5),
            f64::from(height * 0.5),
        );
        state.input.pointer.local = point;
        assert!(input::button(&mut state, MouseButton::Left, true, &mut 0));
        assert_eq!(state.image_window.pressed, Some(hit));
        assert!(input::button(&mut state, MouseButton::Left, false, &mut 0));
        assert_eq!(state.image_window.action.take(), Some(action));
    }
    state.input.pointer.local = (
        f64::from(state.toolbar.size.0 + 12.0),
        f64::from(height * 0.5),
    );
    assert!(input::button(&mut state, MouseButton::Left, true, &mut 0));
    assert_eq!(
        state.image_window.action.take(),
        Some(crate::types::image_window::Action::Move)
    );
    input::button(&mut state, MouseButton::Left, false, &mut 0);
}

/// 【图片窗口测试】【小图平移】缩小图片后中轴仍可移动，两个滚动条隐藏；放大溢出后仅靠近相应边缘显示
#[test]
fn smaller_images_pan_and_overflow_scrollbars_reveal_on_demand() {
    use crate::features::canvas_navigation::{model::Navigation, scrollbars::Scrollbars};
    let mut view = Navigation::new((640, 480), (640, 480), true).unwrap();
    view.set_free_pan(true);
    view.zoom_at(0.5, (320.0, 240.0), true);
    let before = view.projection().widget_rect;
    assert!(view.pan_from(view.center().unwrap(), (320.0, 240.0), (360.0, 260.0), true));
    assert_eq!(view.projection().widget_rect.x, before.x + 40.0);
    let mut bars = Scrollbars::default();
    bars.update(view, true, (639.0, 479.0), (false, false));
    assert!(!bars.horizontal.visible && !bars.vertical.visible);
    view.zoom_at(4.0, (320.0, 240.0), true);
    bars.update(view, true, (320.0, 240.0), (false, false));
    assert!(!bars.horizontal.visible && !bars.vertical.visible);
    bars.update(view, true, (639.0, 479.0), (false, false));
    assert!(bars.horizontal.visible && bars.vertical.visible);
}

/// 【图片窗口测试】【跨边界释放】画布中键平移进入标题栏后释放必须结束，随后移动指针不能继续平移
#[test]
fn releasing_a_canvas_pan_on_the_titlebar_finishes_the_gesture() {
    let mut state = crate::app::input::tests::editor();
    state.file_canvas = FileCanvas::new(
        tiny_skia::Pixmap::new(160, 100).unwrap(),
        state.placements[0].rect().unwrap(),
    );
    state.image_window.enabled = true;
    state.fullscreen_scope = true;
    state.selection.zone = Some(state.file_canvas.as_ref().unwrap().viewport);
    state.ensure_navigation();
    state
        .navigation
        .view
        .as_mut()
        .unwrap()
        .zoom_at(0.5, (200.0, 150.0), true);
    state.navigation.pan = Some(crate::editor::canvas_navigation::state::Pan {
        center: state.navigation.view.unwrap().center().unwrap(),
        start: (200.0, 150.0),
    });
    state.input.pointer.local = (200.0, 20.0);
    state.input.pointer.global = (200.0, 20.0);
    crate::app::input::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Middle,
        false,
        &mut 0,
    );
    assert!(state.navigation.pan.is_none());
    let before = state.canvas_transform();
    crate::app::input::handle_pointer_move(&mut state, 0, 200.0, 200.0, &mut 0);
    assert_eq!(state.canvas_transform(), before);
}

/// 【图片窗口测试】【连续调整】窗口往返调整同时保留原图透明度、选区、历史与原生分辨率导出，无参数、无返回值
#[test]
fn resize_preserves_source_annotations_and_export() {
    let mut state = super::super::region_rendering_test_support::editor(1.25, true);
    let mut original = tiny_skia::Pixmap::new(320, 180).unwrap();
    original.fill(tiny_skia::Color::from_rgba8(120, 50, 90, 130));
    let file = FileCanvas::new(original, Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap()).unwrap();
    state.selection.zone = Some(file.viewport);
    state.file_canvas = Some(file);
    state
        .annotations
        .push(super::super::region_rendering_test_support::annotation(
            41,
            0.0,
            crate::types::AnnotationShape::Rectangle {
                radius: 0.0,
                focus: false,
                fixed_ratio: false,
                start: (80.0, 120.0),
                end: (150.0, 160.0),
                filled: true,
            },
        ));
    let selection = state.selection.zone;
    let annotations = state.annotations.clone();
    let original = state.file_canvas.as_ref().unwrap().original.data().to_vec();
    let exported = super::super::image_export::render_precise(&mut state)
        .unwrap()
        .0;
    for size in [(900, 600), (480, 320), (640, 480)] {
        resize(&mut state, size, &mut 0);
        super::super::region_rendering_test_support::paint(&mut state, None, false);
        assert_eq!(state.placements[0].size, (size.0 as i32, size.1 as i32));
        assert_eq!(state.canvas[0].width(), (size.0 as f32 * 1.25) as u32);
        assert_eq!(state.selection.zone, selection);
        assert_eq!(state.annotations, annotations);
        assert!(state.undo_stack.is_empty());
        assert_eq!(
            state.file_canvas.as_ref().unwrap().original.data(),
            original
        );
        assert_eq!(
            super::super::image_export::render_precise(&mut state)
                .unwrap()
                .0,
            exported
        );
    }
}
