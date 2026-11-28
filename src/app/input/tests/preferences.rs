//! 【设置窗口测试】【截图退出】侧栏入口先结束截图，不再暂停后返回原选区

use super::{annotation_selection::selected, pointer};
use crate::{
    app::input::handle_pointer_button,
    types::MouseButton,
    ui::toolbar::{Integration, ToolbarButton, ToolbarItem},
};

/// 【设置窗口测试】【侧栏输入】按真实侧栏按钮完成左键点击，返回已请求设置并取消截图的编辑器
fn request_from_sidebar() -> crate::editor::EditorState {
    let mut state = selected();
    crate::app::input::refresh_panels(&mut state, &mut 0);
    let panel = &state.action_toolbar;
    let index = panel
        .items
        .iter()
        .position(|item| {
            matches!(
                item,
                ToolbarItem::Button(ToolbarButton::Integration(Integration::Settings))
            )
        })
        .unwrap();
    let cell = panel.cell(index).unwrap();
    let point = (
        f64::from(panel.render_pos.0 + cell.left() + cell.width() * 0.5),
        f64::from(panel.render_pos.1 + cell.top() + cell.height() * 0.5),
    );
    pointer(&mut state, point.0, point.1, &mut 0);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
    assert!(!state.preferences_requested);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
    state
}

/// 【设置窗口测试】【取消导出】侧栏设置点击完成时取消截图并保留设置请求，不生成导出或历史记录
#[test]
fn sidebar_settings_cancel_capture_without_exporting() {
    let state = request_from_sidebar();
    assert!(state.preferences_requested && state.cancel);
    assert!(state.finish.is_none());
    assert!(state.undo_stack.is_empty());
}

/// 【设置窗口测试】【循环退出】生产截图循环在设置请求后立即返回，请求由释放窗口后的外层接收
#[cfg(target_os = "linux")]
#[test]
fn sidebar_settings_leave_the_overlay_loop_for_window_teardown() {
    let mut state = request_from_sidebar();
    let mut overlay: Box<dyn crate::backend::ScreenOverlay> = Box::new(
        crate::app::frame_lifecycle_overlay::Overlay::new(&state.canvas, false),
    );
    assert!(matches!(
        crate::app::event_loop::run_overlay(&mut state, &mut overlay).unwrap(),
        crate::app::event_loop::Exit::Finished
    ));
    assert!(state.preferences_requested && state.cancel);
    assert!(state.finish.is_none());
}
