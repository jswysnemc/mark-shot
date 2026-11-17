//! 【设置回归】【真实控件】覆盖原生窗口事件与业务配置，不以写入文件代替交互验证

use super::{catalog::Page, window::Preferences};
use crate::backend::window::{WindowContent, WindowEvent};

mod autostart;
mod effective;

/// 【设置回归】【整行编辑】无参数，常规和最小窗口下编辑路径不会挤压输入框，绘制和按钮命中共用坐标
#[test]
fn path_editing_uses_full_row_at_all_supported_widths() {
    use super::window::Control;
    for width in [640, 900, 1200] {
        let mut state = Preferences::new(false).unwrap();
        state.size.0 = width;
        state.page = Page::Storage;
        let index = entry("save.directory");
        state.activate(Control::Row(index));
        let layout = super::layout::Layout::new(&state);
        let row = layout.rows.iter().find(|row| row.index == index).unwrap();
        let input = super::layout::editor_rect(row);
        let edit = super::layout::text_edit(row.control);
        assert!(input.width() >= 280.0);
        assert!(input.top() > row.bounds.top() + 50.0);
        for (control, rect) in [
            (Control::Reset(index), edit.reset),
            (Control::Apply, edit.save),
        ] {
            assert_eq!(
                layout.hit(
                    &state,
                    (f64::from(rect.left() + 10.0), f64::from(rect.top() + 10.0))
                ),
                Some(control)
            );
        }
    }
}

/// 【设置回归】【底部展开】无参数，视口底部可见行展开后自动滚动，编辑区及按钮保持完整可见
#[test]
fn editing_a_bottom_row_reveals_its_expanded_input() {
    use super::window::Control;
    let mut state = Preferences::new(false).unwrap();
    state.size = (900, 420);
    state.page = Page::Storage;
    let index = entry("save.directory");
    let layout = super::layout::Layout::new(&state);
    let row = layout.rows.iter().find(|row| row.index == index).unwrap();
    state.scroll = (row.bounds.bottom() - layout.viewport.bottom()).max(0.0) as usize;
    state.activate(Control::Row(index));
    let layout = super::layout::Layout::new(&state);
    let row = layout.rows.iter().find(|row| row.index == index).unwrap();
    assert!(row.control.top() >= layout.viewport.top());
    assert!(row.control.bottom() <= layout.viewport.bottom());
}

/// 【设置回归】【全局绑定】无参数，全局快捷键按单个组合保存，清除只禁用全局监听，不修改编辑器绑定
#[test]
fn global_shortcut_records_one_chord_without_replacing_editor_bindings() {
    use super::window::Control;
    let mut state = Preferences::new(false).unwrap();
    let index = entry("desktop.screenshot_shortcut");
    state.activate(Control::Row(index));
    state.record_shortcut(
        index,
        &crate::backend::window::KeyInput {
            key: Some(crate::keys::Key::PrintScreen),
            text: String::new(),
            mods: crate::keys::Mods {
                ctrl: true,
                ..Default::default()
            },
        },
    );
    assert_eq!(
        state.values["desktop"]["screenshot_shortcut"],
        "Ctrl+PrintScreen"
    );
    assert!(!state.save(index, "Ctrl+InvalidKey".into()));
    assert!(!state.save(index, serde_json::json!(["F8", "F9"])));
    assert!(state.save_field("desktop", "shortcuts_enabled", true.into()));
    state.activate(Control::Clear(index));
    assert_eq!(state.values["desktop"]["shortcuts_enabled"], false);
    assert_eq!(
        state.values["desktop"]["screenshot_shortcut"],
        "Ctrl+PrintScreen"
    );
    assert_eq!(state.value(entry("keys.copy")), "Ctrl+C");
}

/// 【设置回归】【颜色事务】无参数，通过真实对话框事件验证取消、透明色确认与恢复默认
#[test]
fn shared_color_dialog_preserves_alpha_and_commits_only_explicit_choices() {
    use super::window::Control;
    use crate::backend::window::{KeyInput, LEFT_BUTTON};
    let mut state = Preferences::new(false).unwrap();
    let index = entry("pinned.border_color");
    assert!(state.save(index, "#12345680".into()));
    state.activate(Control::Row(index));
    let mut dialog = state.take_dialog().unwrap();
    dialog.content.event(WindowEvent::Key(KeyInput {
        key: Some(crate::keys::Key::Escape),
        text: String::new(),
        mods: Default::default(),
    }));
    state.dialog_closed(dialog.id, false);
    assert_eq!(state.value(index), "#12345680");
    state.activate(Control::Row(index));
    let mut dialog = state.take_dialog().unwrap();
    dialog.content.event(WindowEvent::PointerButton {
        position: (75.0, 207.0),
        button: LEFT_BUTTON,
        pressed: true,
    });
    dialog.content.event(WindowEvent::Key(KeyInput {
        key: None,
        text: "#ff000040".into(),
        mods: Default::default(),
    }));
    dialog.content.event(WindowEvent::Key(KeyInput {
        key: Some(crate::keys::Key::Enter),
        text: String::new(),
        mods: Default::default(),
    }));
    assert!(dialog.content.dialog_accepted());
    state.dialog_closed(dialog.id, true);
    assert_eq!(state.value(index), "#FF000040");
    state.activate(Control::Row(index));
    let mut dialog = state.take_dialog().unwrap();
    for pressed in [true, false] {
        dialog.content.event(WindowEvent::PointerButton {
            position: (30.0, 300.0),
            button: LEFT_BUTTON,
            pressed,
        });
    }
    state.dialog_closed(dialog.id, true);
    assert_eq!(
        state.value(index),
        super::catalog::ENTRIES[index]
            .display(&serde_json::to_value(crate::config::Config::default()).unwrap())
    );
}

/// 【设置回归】【实际色块】无参数，设置页在控件位置绘制所选颜色，不以主题色或色值文本代替
#[test]
fn color_setting_renders_its_actual_rgb_value() {
    let mut state = Preferences::new(false).unwrap();
    state.page = Page::Pinned;
    let index = entry("pinned.border_color");
    state.values["pinned"]["border_color"] = "#f20c38ff".into();
    let layout = super::layout::Layout::new(&state);
    let row = layout.rows.iter().find(|row| row.index == index).unwrap();
    let sample = (
        (row.control.left() + 16.0) as u32,
        (row.control.top() + 12.0) as u32,
    );
    let mut image = tiny_skia::Pixmap::new(state.size.0, state.size.1).unwrap();
    state.draw(&mut image, 1.0);
    let color = image.pixel(sample.0, sample.1).unwrap().demultiply();
    assert_eq!((color.red(), color.green(), color.blue()), (242, 12, 56));
}

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

/// 【设置回归】【滚轮数值】无参数，数值框滚轮先提交草稿再调整，边界不滚动页面，框外保留页面滚动
#[test]
fn wheel_over_number_changes_value_and_commits_the_edit() {
    use super::{catalog::ENTRIES, layout::Layout, window::Control};
    let mut state = Preferences::new(false).unwrap();
    let index = ENTRIES
        .iter()
        .position(|entry| entry.section == "history" && entry.key == "limit")
        .unwrap();
    state.page = ENTRIES[index].page;
    state.activate(Control::Row(index));
    state
        .editing
        .as_mut()
        .unwrap()
        .1
        .set("50", &mut state.fonts, true);
    let layout = Layout::new(&state);
    let row = layout.rows.iter().find(|row| row.index == index).unwrap();
    let position = (
        f64::from(row.control.left() + row.control.width() / 2.0),
        f64::from(row.control.top() + 10.0),
    );
    let before = state.scroll;
    state.event(WindowEvent::Scroll {
        position,
        delta: (0.0, -15.0),
    });
    let step = ENTRIES[index].step().unwrap();
    assert_eq!(state.value(index).parse::<f64>().unwrap(), 50.0 + step);
    assert_eq!(state.scroll, before);
    assert!(state.editing.is_none());
}

/// 【设置回归】【中文输入法】验证顶部搜索框、行内编辑框与可搜索下拉框均导出输入法身份、光标矩形并接收提交文本
#[test]
fn all_settings_input_fields_expose_ime_targets_and_accept_committed_chinese_text() {
    use super::window::Control;
    use crate::types::text_input::{ChangeCause, Edit, Focus, Preedit};

    let focus = Focus {
        id: 7,
        monitor_idx: 0,
    };
    let mut state = Preferences::new(false).unwrap();
    assert!(state.text_input_target().is_none());

    // 1. 【设置回归】【顶部搜索】空搜索框获得焦点后立即提供有效光标矩形并接受中文提交
    state.activate(Control::Search);
    let search_id = state.text_input_target().expect("search bar IME target");
    let search_snap = state
        .text_input_snapshot(focus, search_id, None, ChangeCause::Other)
        .expect("search bar IME snapshot");
    assert!(search_snap.cursor_rect.is_some());
    state.event(WindowEvent::TextInput(Edit {
        preedit: Preedit::default(),
        commit: "保存目录".into(),
        replacement: None,
    }));
    assert_eq!(state.search.text(), "保存目录");
    state.search.set("", &mut state.fonts, false);
    state.search_focus = false;

    // 2. 【设置回归】【行内编辑】设置项单行文本框导出输入法快照并可保存中文路径
    state.page = Page::Storage;
    let directory = entry("save.directory");
    state.activate(Control::Row(directory));
    let row_id = state.text_input_target().expect("row editor IME target");
    assert_ne!(row_id, search_id);
    let row_snap = state
        .text_input_snapshot(focus, row_id, None, ChangeCause::Other)
        .expect("row editor IME snapshot");
    assert!(row_snap.cursor_rect.is_some());
    state.event(WindowEvent::TextInput(Edit {
        preedit: Preedit::default(),
        commit: "/tmp/截图目录".into(),
        replacement: None,
    }));
    assert!(state.apply());
    assert_eq!(state.value(directory), "/tmp/截图目录");

    // 3. 【设置回归】【下拉搜索】超过八项的可搜索下拉菜单在英文与中文界面下均接收中文输入法并实时过滤选项
    for language in ["english", "chinese"] {
        state.dropdown = None;
        state.values["ui"]["language"] = language.into();
        state.page = Page::Capture;
        let tool = entry("startup.normal_tool");
        state.activate(Control::Row(tool));
        let dropdown_id = state
            .text_input_target()
            .expect("searchable dropdown IME target");
        let dropdown_snap = state
            .text_input_snapshot(focus, dropdown_id, None, ChangeCause::Other)
            .expect("searchable dropdown IME snapshot");
        assert!(dropdown_snap.cursor_rect.is_some());
        state.event(WindowEvent::TextInput(Edit {
            preedit: Preedit::default(),
            commit: "文字".into(),
            replacement: None,
        }));
        let menu = state.dropdown.as_ref().unwrap();
        assert_eq!(
            menu.search.as_ref().map(|field| field.text()).as_deref(),
            Some("文字")
        );
        assert!(
            menu.filtered
                .iter()
                .any(|&idx| menu.options[idx].value == "text")
        );
    }
}
