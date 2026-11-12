//! 【设置导航测试】【子页面返回】通过真实控件事件验证返回入口和搜索上下文恢复

use crate::{
    backend::window::{KeyInput, LEFT_BUTTON, WindowAction, WindowContent, WindowEvent},
    features::preferences::{
        catalog::{ENTRIES, Page},
        layout,
        window::{Control, Preferences},
    },
    keys::Key,
};

/// 【设置导航测试】【配置入口】无参数，返回工具栏自定义入口的目录下标
fn entry() -> usize {
    ENTRIES
        .iter()
        .position(|entry| entry.section == "toolbar_layout" && entry.key == "layout")
        .unwrap()
}

/// 【设置导航测试】【返回按钮】按 position 向 state 交付完整左键点击，返回释放动作
fn click(state: &mut Preferences, position: (f64, f64)) -> WindowAction {
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

/// 【设置导航测试】【可见返回】进入工具栏编辑后，标题旁的返回按钮恢复普通设置而不关闭窗口
#[test]
fn toolbar_editor_has_a_clickable_return_control() {
    let mut state = Preferences::new(false).unwrap();
    state.activate(Control::Page(Page::Toolbar));
    state.activate(Control::Row(entry()));
    assert!(state.toolbar_page());
    let point = (f64::from(layout::sidebar(&state) + 54.0), 80.0);
    assert!(
        state.hit(point).is_some(),
        "toolbar editor has no visible return control"
    );
    assert_ne!(click(&mut state, point), WindowAction::Close);
    assert!(!state.toolbar_page());
    assert!(state.page == Page::Toolbar);
    assert_eq!(state.focus, Some(entry()));
}

/// 【设置导航测试】【搜索恢复】从搜索结果进入工具栏编辑后，Esc 恢复原搜索分类和入口焦点
#[test]
fn escape_restores_the_search_that_opened_toolbar_editor() {
    let mut state = Preferences::new(false).unwrap();
    let page = state.page;
    state.search.set("toolbar_layout", &mut state.fonts, false);
    state.activate(Control::Row(entry()));
    assert!(state.toolbar_page());
    let action = state.event(WindowEvent::Key(KeyInput {
        key: Some(Key::Escape),
        text: String::new(),
        mods: Default::default(),
    }));
    assert_ne!(action, WindowAction::Close);
    assert!(!state.toolbar_page());
    assert!(state.page == page);
    assert_eq!(state.search.text(), "toolbar_layout");
    assert_eq!(state.focus, Some(entry()));
}
