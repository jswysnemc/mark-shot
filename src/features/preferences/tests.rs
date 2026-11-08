//! 【设置回归】【真实控件】覆盖原生窗口事件与业务配置，不以写入文件代替交互验证

use super::{catalog::Page, window::Preferences};
use crate::backend::window::{WindowContent, WindowEvent};

/// 【设置回归】【字段定位】按 path 查找注册项，返回目录索引，缺失时使测试失败
fn entry(path: &str) -> usize {
    super::catalog::ENTRIES
        .iter()
        .position(|entry| entry.path() == path)
        .unwrap()
}

/// 【设置回归】【旧语言显示】无参数，旧顶层语言与新键使用同一显示优先级
#[test]
fn legacy_language_is_shown_until_overridden() {
    let mut state = Preferences::new(false).unwrap();
    state.values["language"] = "chinese".into();
    state.values["ui"]["language"] = "".into();
    assert_eq!(state.value(entry("ui.language")), "chinese");
    state.values["ui"]["language"] = "english".into();
    assert_eq!(state.value(entry("ui.language")), "english");
}

/// 【设置回归】【主题唤醒】无参数，翻译页没有待完成模型查询时也响应系统主题通知
#[test]
fn translation_page_redraws_on_system_wake() {
    let mut state = Preferences::new(false).unwrap();
    state.page = Page::Translation;
    assert!(matches!(
        state.event(WindowEvent::Wake),
        crate::backend::window::WindowAction::Redraw
    ));
}

/// 【设置回归】【提交与关闭】无参数，切页保存文字，非法数值阻止关闭且保留草稿，修正后正常提交
#[test]
fn pending_values_commit_on_navigation_and_close() {
    use super::window::Control;
    let mut state = Preferences::new(false).unwrap();
    let directory = entry("save.directory");
    state.activate(Control::Row(directory));
    state
        .editing
        .as_mut()
        .unwrap()
        .1
        .set("/tmp/capture-example", &mut state.fonts, false);
    state.activate(Control::Page(Page::Pinned));
    assert_eq!(state.values["save"]["directory"], "/tmp/capture-example");
    let radius = entry("export.image_frame.corner_radius");
    state.activate(Control::Row(radius));
    state
        .editing
        .as_mut()
        .unwrap()
        .1
        .set("129", &mut state.fonts, false);
    assert!(!state.close_requested());
    assert!(state.editing.is_some());
    state
        .editing
        .as_mut()
        .unwrap()
        .1
        .set("20.5", &mut state.fonts, false);
    assert!(state.close_requested());
    assert_eq!(state.values["export"]["image_frame"]["corner_radius"], 20.5);
}

/// 【设置回归】【数值预览】无参数，有效草稿即时更新预览但不写文件，越界草稿不改变预览参数
#[test]
fn frame_drafts_preview_without_writing_and_step_with_bounds() {
    use super::window::Control;
    let mut state = Preferences::new(false).unwrap();
    state.page = Page::Pinned;
    let padding = entry("export.image_frame.padding");
    state.activate(Control::Row(padding));
    state
        .editing
        .as_mut()
        .unwrap()
        .1
        .set("200", &mut state.fonts, false);
    assert_eq!(super::frame_preview::options(&state).padding, 200);
    assert!(!state.path.exists());
    state
        .editing
        .as_mut()
        .unwrap()
        .1
        .set("300", &mut state.fonts, false);
    assert_eq!(super::frame_preview::options(&state).padding, 112);
    state
        .editing
        .as_mut()
        .unwrap()
        .1
        .set("256", &mut state.fonts, false);
    state.activate(Control::Step(padding, 1));
    assert_eq!(state.values["export"]["image_frame"]["padding"], 256);
    state.activate(Control::Reset(padding));
    assert_eq!(state.values["export"]["image_frame"]["padding"], 112);
    for size in [(580, 420), (900, 640)] {
        state.size = size;
        let layout = super::layout::Layout::new(&state);
        let row = layout.rows.iter().find(|row| row.index == padding).unwrap();
        assert!(!row.stacked);
        assert!(row.control.width() <= 132.0 && row.bounds.height() <= 44.0);
        assert!(layout.preview.is_some());
    }
}

/// 【设置回归】【错误配置入口】无参数，语法错误仍能创建设置窗口并展示恢复入口
#[test]
fn malformed_configuration_opens_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    std::fs::write(&path, "[save]\ndirectory=\"broken\n").unwrap();
    let mut state = Preferences::from_path(path, false).unwrap();
    assert!(state.recovery.is_some());
    state.repair();
    assert!(state.recovery.is_none());
}

/// 【设置回归】【快捷键追加】无参数，录入保留重做已有两组绑定，清除与恢复具有独立动作
#[test]
fn shortcut_recording_keeps_existing_bindings() {
    use super::window::Control;
    use crate::{
        backend::window::KeyInput,
        keys::{Key, Mods},
    };
    let mut state = Preferences::new(false).unwrap();
    let redo = entry("keys.redo");
    let previous = state.value(redo);
    state.activate(Control::Row(redo));
    state.event(WindowEvent::Key(KeyInput {
        key: Some(Key::Char('j')),
        text: String::new(),
        mods: Mods {
            ctrl: true,
            alt: true,
            ..Default::default()
        },
    }));
    assert!(state.value(redo).starts_with(&previous));
    assert_ne!(state.value(redo), previous);
    state.activate(Control::Clear(redo));
    assert!(state.value(redo).is_empty());
    state.activate(Control::Reset(redo));
    assert_eq!(state.value(redo), previous);
}

/// 【设置回归】【滚动条命中】无参数，窄窗口内容溢出时右侧轨道必须可操作，拖到底部能访问末项
#[test]
fn overflow_has_a_draggable_scrollbar() {
    let mut state = Preferences::new(false).unwrap();
    state.page = Page::Capture;
    state.event(WindowEvent::Resize { size: (580, 420) });
    let bar = state.scrollbar().unwrap();
    let point = (
        f64::from(bar.handle.left() + 2.0),
        f64::from(bar.handle.top() + 3.0),
    );
    assert!(
        state.hit(point).is_some(),
        "overflow has no scrollbar hit target"
    );
    state.event(WindowEvent::PointerButton {
        position: point,
        button: 0x110,
        pressed: true,
    });
    state.event(WindowEvent::PointerMove {
        position: (point.0, 1000.0),
    });
    state.event(WindowEvent::PointerButton {
        position: (point.0, 1000.0),
        button: 0x110,
        pressed: false,
    });
    assert_eq!(state.scroll, super::layout::Layout::new(&state).maximum);
}

/// 【设置回归】【滚轮翻页】无参数，原生滚轮与键盘可到达首尾，放大窗口后滚动条消失
#[test]
fn wheel_and_keyboard_reach_all_settings() {
    use crate::{
        backend::window::{KeyInput, ScrollInput},
        keys::Key,
    };
    let mut state = Preferences::new(false).unwrap();
    state.page = Page::Capture;
    state.event(WindowEvent::Resize { size: (580, 420) });
    state.scroll(ScrollInput::legacy((300.0, 220.0), (0.0, 10.0)));
    assert_eq!(state.scroll, 64);
    state.event(WindowEvent::Key(KeyInput {
        key: Some(Key::End),
        text: String::new(),
        mods: Default::default(),
    }));
    assert_eq!(state.scroll, super::layout::Layout::new(&state).maximum);
    state.event(WindowEvent::Resize { size: (900, 1800) });
    assert!(state.scrollbar().is_none());
    assert_eq!(state.scroll, 0);
}

/// 【设置回归】【键盘可达】无参数，验证分组和窄窗口叠放后 Tab 可以依次访问所有字段且不会停在裁切区
#[test]
fn tab_keeps_each_grouped_field_visible_in_compact_window() {
    use crate::{backend::window::KeyInput, keys::Key};
    let mut state = Preferences::new(false).unwrap();
    state.page = Page::Capture;
    state.event(WindowEvent::Resize { size: (580, 420) });
    for index in state.visible() {
        state.event(WindowEvent::Key(KeyInput {
            key: Some(Key::Tab),
            text: String::new(),
            mods: Default::default(),
        }));
        assert_eq!(state.focus, Some(index));
        let layout = super::layout::Layout::new(&state);
        let row = layout.rows.iter().find(|row| row.index == index).unwrap();
        assert!(row.bounds.top() >= layout.viewport.top());
        assert!(row.bounds.bottom() <= layout.viewport.bottom());
    }
}
