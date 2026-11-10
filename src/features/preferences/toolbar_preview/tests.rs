//! 【工具栏预览测试】【即时更新】验证真实下拉保存、示意图变化及窄窗口滚动边界

use super::*;
use crate::{
    backend::window::{LEFT_BUTTON, WindowAction, WindowContent, WindowEvent},
    features::preferences::{
        catalog::{ENTRIES, Page},
        layout::Layout,
        window::{Control, Preferences},
    },
};

/// 【工具栏预览测试】【选择事件】按 state、index 和 value 通过下拉指针事件保存设置，返回动作供重绘断言
fn choose(state: &mut Preferences, index: usize, value: &str) -> WindowAction {
    state.activate(Control::Row(index));
    let menu = state.dropdown.as_ref().unwrap();
    let option = menu
        .options
        .iter()
        .position(|item| item.value == value)
        .unwrap();
    let layout = crate::features::preferences::dropdown::layout::Layout::new(menu, state.size);
    let bounds = layout
        .rows
        .iter()
        .find(|(index, _)| *index == option)
        .unwrap()
        .1;
    let position = (
        f64::from(bounds.left() + bounds.width() / 2.0),
        f64::from(bounds.top() + bounds.height() / 2.0),
    );
    state.event(WindowEvent::PointerButton {
        position,
        button: LEFT_BUTTON,
        pressed: true,
    });
    state.event(WindowEvent::PointerButton {
        position,
        button: LEFT_BUTTON,
        pressed: false,
    })
}

/// 【工具栏预览测试】【真实选择】无参数，五个位置通过下拉保存后立即重绘不同预览，读取原文件仍为所选位置
#[test]
fn toolbar_preview_updates_after_every_position_choice() {
    let mut state = Preferences::new(false).unwrap();
    state.page = Page::Toolbar;
    let index = ENTRIES
        .iter()
        .position(|entry| entry.path() == "capture.toolbar_position")
        .unwrap();
    let mut images = Vec::new();
    for value in ["selection", "top", "bottom", "left", "right"] {
        assert!(matches!(
            choose(&mut state, index, value),
            WindowAction::Redraw
        ));
        assert!(state.dropdown.is_none());
        assert_eq!(state.values["capture"]["toolbar_position"], value);
        let layout = Layout::new(&state);
        let bounds = layout.toolbar_preview.unwrap();
        let mut image = tiny_skia::Pixmap::new(state.size.0, state.size.1).unwrap();
        state.draw(&mut image, 1.0);
        let crop = image
            .clone_rect(
                tiny_skia::IntRect::from_xywh(
                    bounds.left() as i32,
                    bounds.top() as i32,
                    bounds.width() as u32,
                    bounds.height() as u32,
                )
                .unwrap(),
            )
            .unwrap();
        assert!(!images.contains(&crop));
        images.push(crop);
        let saved = crate::config::preferences::load(&state.path).unwrap();
        assert_eq!(saved.capture.toolbar_position, position(&state.values));
    }
}

/// 【工具栏预览测试】【布局边界】无参数，窄窗口、搜索结果与滚动状态都为预览保留独立空间，示意栏不重叠
#[test]
fn toolbar_preview_fits_without_covering_setting_controls() {
    let mut state = Preferences::new(false).unwrap();
    state.page = Page::Toolbar;
    for size in [(580, 420), (900, 640)] {
        state.size = size;
        for search in ["", "toolbar_position"] {
            state.search.set(search, &mut state.fonts, false);
            state.scroll = 0;
            let layout = Layout::new(&state);
            let bounds = layout.toolbar_preview.unwrap();
            assert!(
                layout
                    .rows
                    .iter()
                    .all(|row| !crate::utils::rects_overlap(&bounds, &row.bounds))
            );
            for position in [
                Position::Selection,
                Position::Top,
                Position::Bottom,
                Position::Left,
                Position::Right,
            ] {
                let g = geometry(bounds, position);
                for area in [g.selection, g.main, g.side, g.properties] {
                    assert!(area.left() >= g.screen.left() && area.right() <= g.screen.right());
                    assert!(area.top() >= g.screen.top() && area.bottom() <= g.screen.bottom());
                }
                for (a, b) in [
                    (g.main, g.side),
                    (g.main, g.properties),
                    (g.side, g.properties),
                ] {
                    assert!(!crate::utils::rects_overlap(&a, &b));
                }
            }
            state.scroll = layout.maximum;
            let scrolled = Layout::new(&state);
            assert!(scrolled.toolbar_preview.unwrap().bottom() <= scrolled.viewport.bottom());
        }
    }
}
