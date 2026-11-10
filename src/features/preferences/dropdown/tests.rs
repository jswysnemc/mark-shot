//! 【设置下拉测试】【真实事件】验证选择保存、过滤、取消及窗口边缘定位

use super::{
    Dropdown, OptionItem, Target,
    layout::{Hit, Layout},
};
use crate::{
    backend::window::{KeyInput, LEFT_BUTTON, WindowContent, WindowEvent},
    features::preferences::{
        catalog::{ENTRIES, Page},
        translation::{State, fields::Field},
        window::Preferences,
    },
    keys::Key,
};

/// 【设置下拉测试】【隔离配置】按 target 创建临时翻译设置窗口，返回目录和状态，避免写入用户配置
fn fixture(target: &str) -> (tempfile::TempDir, Preferences) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    std::fs::write(&path, serde_json::json!({"ui":{"language":"chinese"},"translation":{"provider":"plugin:bilibili","target_language":target}}).to_string()).unwrap();
    let mut state = Preferences::new(false).unwrap();
    state.path = path;
    state.values = crate::config::preferences::read(&state.path).unwrap();
    state.translation = State::new(&state.values);
    state.page = Page::Translation;
    state.size = (580, 420);
    (directory, state)
}

/// 【设置下拉测试】【键盘事件】向 state 发送 key 和 text，返回空值
fn key(state: &mut Preferences, key: Option<Key>, text: &str) {
    state.event(WindowEvent::Key(KeyInput {
        key,
        text: text.into(),
        mods: Default::default(),
    }));
}

/// 【设置下拉测试】【字段点击】按 state 当前目标语言框发送真实鼠标事件，无返回值
fn open_target(state: &mut Preferences) {
    // 1. 【设置下拉测试】【字段可见】先通过实际滚轮事件显示语言字段，避免新增预览区后点击视口外坐标
    for _ in 0..24 {
        let layout = crate::features::preferences::translation::layout::Layout::new(state);
        let viewport = crate::features::preferences::layout::viewport(state);
        let (_, bounds) = layout
            .controls
            .iter()
            .find(|(hit, _)| {
                *hit == crate::features::preferences::translation::layout::Hit::Field(Field::Target)
            })
            .unwrap();
        if bounds.top() >= viewport.top() && bounds.bottom() <= viewport.bottom() {
            break;
        }
        state.event(WindowEvent::Scroll {
            position: (
                f64::from(viewport.left() + 1.0),
                f64::from(viewport.top() + 10.0),
            ),
            delta: (
                0.0,
                if bounds.top() < viewport.top() {
                    -1.0
                } else {
                    1.0
                },
            ),
        });
    }
    let layout = crate::features::preferences::translation::layout::Layout::new(state);
    let bounds = layout
        .controls
        .iter()
        .find(|(hit, _)| {
            *hit == crate::features::preferences::translation::layout::Hit::Field(Field::Target)
        })
        .unwrap()
        .1;
    let position = (
        f64::from(bounds.left() + 10.0),
        f64::from(bounds.top() + 16.0),
    );
    for pressed in [true, false] {
        state.event(WindowEvent::PointerButton {
            position,
            button: LEFT_BUTTON,
            pressed,
        });
    }
    assert!(state.dropdown.is_some());
}

/// 【设置下拉测试】【语言选择】无参数，验证搜索中文后确认保存稳定语言值，刷新读取保持一致
#[test]
fn language_search_saves_stable_value_and_reopens_selected() {
    let (_directory, mut state) = fixture("Simplified Chinese");
    open_target(&mut state);
    let before = Layout::new(state.dropdown.as_ref().unwrap(), state.size).bounds;
    key(&mut state, None, "日语");
    let menu = state.dropdown.as_ref().unwrap();
    assert_eq!(Layout::new(menu, state.size).bounds, before);
    assert_eq!(menu.filtered.len(), 1);
    assert_eq!(menu.options[menu.cursor.unwrap()].value, "Japanese");
    key(&mut state, Some(Key::Enter), "");
    assert!(state.dropdown.is_none());
    assert_eq!(
        crate::config::preferences::read(&state.path).unwrap()["translation"]["target_language"],
        "Japanese"
    );
    open_target(&mut state);
    let menu = state.dropdown.as_ref().unwrap();
    assert_eq!(menu.options[menu.selected.unwrap()].label, "日语");
}

/// 【设置下拉测试】【失败重试】无参数，验证保存失败保留候选和旧配置，恢复路径后可以重新确认
#[test]
fn failed_save_keeps_the_popup_and_original_value() {
    let (directory, mut state) = fixture("zh-CN");
    open_target(&mut state);
    let menu = state.dropdown.as_ref().unwrap();
    assert_eq!(menu.options[menu.selected.unwrap()].label, "简体中文");
    key(&mut state, None, "Japanese");
    let path = state.path.clone();
    state.path = directory.path().to_path_buf();
    key(&mut state, Some(Key::Enter), "");
    assert!(state.dropdown.is_some());
    assert_eq!(
        crate::config::preferences::read(&path).unwrap()["translation"]["target_language"],
        "zh-CN"
    );
    state.path = path;
    key(&mut state, Some(Key::Enter), "");
    assert!(state.dropdown.is_none());
    assert_eq!(state.values["translation"]["target_language"], "Japanese");
}

/// 【设置下拉测试】【取消与空结果】无参数，验证未知语言保留，空结果回车、外部点击和 Esc 不修改配置
#[test]
fn custom_value_and_empty_search_never_save_accidentally() {
    let (_directory, mut state) = fixture("Portuguese (Brazil)");
    let before = std::fs::read(&state.path).unwrap();
    open_target(&mut state);
    let menu = state.dropdown.as_ref().unwrap();
    assert_eq!(
        menu.options[menu.selected.unwrap()].value,
        "Portuguese (Brazil)"
    );
    key(&mut state, None, "no-such-language");
    key(&mut state, Some(Key::Enter), "");
    assert!(state.dropdown.as_ref().unwrap().cursor.is_none());
    key(&mut state, Some(Key::Escape), "");
    assert!(state.dropdown.is_none());
    open_target(&mut state);
    state.event(WindowEvent::PointerButton {
        position: (30.0, 75.0),
        button: LEFT_BUTTON,
        pressed: true,
    });
    state.event(WindowEvent::PointerButton {
        position: (30.0, 75.0),
        button: LEFT_BUTTON,
        pressed: false,
    });
    assert!(state.page == Page::Translation);
    assert!(state.dropdown.is_none());
    assert_eq!(std::fs::read(&state.path).unwrap(), before);
}

/// 【设置下拉测试】【共享选项】无参数，验证普通设置使用相同组件并通过键盘保存，窗口失焦仅关闭浮层
#[test]
fn setting_choices_share_commit_and_dismiss_behavior() {
    let (_directory, mut state) = fixture("English");
    state.page = Page::Appearance;
    let index = ENTRIES
        .iter()
        .position(|entry| entry.section == "ui" && entry.key == "language")
        .unwrap();
    crate::features::preferences::menus::setting(&mut state, index);
    assert!(state.dropdown.as_ref().unwrap().search.is_none());
    key(&mut state, Some(Key::End), "");
    key(&mut state, Some(Key::Enter), "");
    assert_eq!(state.values["ui"]["language"], "english");
    crate::features::preferences::menus::setting(&mut state, index);
    state.event(WindowEvent::Focused(false));
    assert!(state.dropdown.is_none());
}

/// 【设置下拉测试】【滚动与边界】无参数，验证长列表在窗口两端保持完整，滚动后命中对应真实选项
#[test]
fn long_lists_fit_edges_and_hit_scrolled_options() {
    let (_directory, mut state) = fixture("English");
    for top in [60.0, 205.0, 345.0] {
        let options = (0..30)
            .map(|index| OptionItem::new(index.to_string(), format!("Model {index}"), ""))
            .collect();
        let mut menu = Dropdown::new(
            Target::Translation(Field::Model),
            crate::features::preferences::layout::rect(370.0, top, 180.0, 32.0),
            options,
            "29",
            &mut state.fonts,
        );
        menu.reveal(Layout::new(&menu, state.size).capacity);
        let layout = Layout::new(&menu, state.size);
        assert!(layout.bounds.top() >= 52.0 && layout.bounds.bottom() <= 382.0);
        assert!(layout.bounds.left() >= 12.0 && layout.bounds.right() <= 568.0);
        let (index, bounds) = layout.rows.last().unwrap();
        assert_eq!(*index, 29);
        assert!(
            layout.hit((
                f64::from(bounds.left() + 10.0),
                f64::from(bounds.top() + 16.0)
            )) == Some(Hit::Option(29))
        );
    }
}
