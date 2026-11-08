//! 【翻译设置测试】【表单持久化】验证协议独立配置、错误恢复与紧凑布局

use super::{fields::Field, state::State};
use serde_json::json;

/// 【翻译设置测试】【保存与隔离】无参数，验证 JSON/TOML 保存及非法正文不改写文件，返回空值
#[test]
fn preferences_roundtrip_and_reject_invalid_json() {
    for extension in ["toml", "json"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(format!("config.{extension}"));
        let source = if extension == "toml" {
            "# keep comment\n[ui]\nlanguage = 'zh-CN'\n[translation]\nprovider = 'plugin:bilibili'\n[translation.anthropic]\napiKey = 'other-secret'\n"
        } else {
            r#"{"ui":{"language":"zh-CN"},"translation":{"provider":"plugin:bilibili","anthropic":{"apiKey":"other-secret"}}}"#
        };
        std::fs::write(&path, source).unwrap();
        let values = crate::config::preferences::read(&path).unwrap();
        let mut state = State::new(&values);
        assert_eq!(state.selected, 0);
        crate::config::translation::preferences::save(
            &path,
            &serde_json::from_value(json!({"bilibili":{"unknown":"preserve"}})).unwrap(),
        )
        .unwrap();
        state
            .save(&path, Field::Model, "Index-Translate-2B")
            .unwrap();
        state
            .save(
                &path,
                Field::Extra,
                r#"{"temperature":null,"chat_template_kwargs":{"enable_thinking":false}}"#,
            )
            .unwrap();
        state.save(&path, Field::Timeout, "12000").unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(state.save(&path, Field::Extra, "[]").is_err());
        assert!(state.save(&path, Field::Timeout, "1").is_err());
        assert!(
            state
                .save(&path, Field::Address, "https://user:key@example.com")
                .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let root = crate::config::preferences::read(&path).unwrap();
        assert_eq!(root["translation"]["anthropic"]["apiKey"], "other-secret");
        assert_eq!(root["translation"]["bilibili"]["unknown"], "preserve");
        let service =
            crate::translation::resolve::service(&root["translation"], "plugin:bilibili", &|_| {
                None
            })
            .unwrap();
        let crate::translation::resolve::Service::Llm(config) = service else {
            panic!()
        };
        assert_eq!(config.model, "Index-Translate-2B");
        assert_eq!(config.timeout.as_millis(), 12000);
        assert!(config.extra_body["temperature"].is_null());
        state.root["command"] = json!("old-command");
        state.select(&path, 2).unwrap();
        assert_eq!(state.root["command"], "old-command");
        assert_eq!(state.root["provider"], "plugin:bilibili");
        state.enable(&path, false).unwrap();
        assert_eq!(state.root["command"], "");
        assert_eq!(state.root["provider"], "plugin:openai-responses");
        if extension == "toml" {
            assert!(
                std::fs::read_to_string(&path)
                    .unwrap()
                    .contains("# keep comment")
            );
        }
    }
}

/// 【翻译设置测试】【字段范围】无参数，验证机器翻译不显示模型字段，窗口最小时导航不重叠
#[test]
fn machine_services_hide_model_fields_and_navigation_fits() {
    let mut state = State::new(&json!({"translation":{"provider":"plugin:tencent-tmt"}}));
    assert!(!state.fields().contains(&Field::Models));
    assert!(state.fields().contains(&Field::AppId));
    state.selected = 4;
    assert!(state.fields().contains(&Field::Extra));
    let mut preferences = super::super::window::Preferences::new(false).unwrap();
    preferences.size = (580, 420);
    for index in 1..super::super::catalog::Page::ALL.len() {
        assert!(
            super::super::layout::nav_rect(&preferences, index - 1).bottom()
                <= super::super::layout::nav_rect(&preferences, index).top()
        );
    }
}

/// 【翻译设置测试】【卡片与字段】无参数，通过窗口点击选择服务并提交模型，验证重排后的命中和配置隔离
#[test]
fn service_cards_and_scrolled_fields_persist_through_window_events() {
    use super::layout::{Hit, Layout};
    use crate::backend::window::{KeyInput, LEFT_BUTTON, WindowContent, WindowEvent};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    std::fs::write(&path, "[translation]\nprovider = 'plugin:bilibili'\n[translation.bilibili]\nmodel = 'keep-model'\n").unwrap();
    let mut preferences = super::super::window::Preferences::new(false).unwrap();
    preferences.path = path;
    preferences.values = crate::config::preferences::read(&preferences.path).unwrap();
    preferences.translation = State::new(&preferences.values);
    preferences.page = super::super::catalog::Page::Translation;
    assert!(super::input::choose(&mut preferences, Field::Provider, "1"));
    for hit in [Hit::Field(Field::Model)] {
        let layout = Layout::new(&preferences);
        if let Hit::Field(field) = hit {
            let row = layout.rows.iter().find(|row| row.field == field).unwrap();
            preferences.translation.scroll =
                (row.bounds.top() - layout.viewport.bottom() + 12.0).max(0.0) as usize;
        }
        let layout = Layout::new(&preferences);
        let bounds = layout
            .controls
            .iter()
            .find(|(control, _)| *control == hit)
            .unwrap()
            .1;
        let position = (
            f64::from(bounds.left() + bounds.width() / 2.0),
            f64::from(bounds.top() + 6.0),
        );
        assert!(layout.hit(position) == Some(hit));
        for pressed in [true, false] {
            preferences.event(WindowEvent::PointerButton {
                position,
                button: LEFT_BUTTON,
                pressed,
            });
        }
    }
    let layout = Layout::new(&preferences);
    let row = layout
        .rows
        .iter()
        .find(|row| row.field == Field::Model)
        .unwrap();
    assert!(row.bounds.bottom() <= layout.viewport.bottom());
    assert!(row.label.top() >= layout.viewport.top());
    preferences.event(WindowEvent::Key(KeyInput {
        key: None,
        text: "custom-model".into(),
        mods: Default::default(),
    }));
    preferences.event(WindowEvent::Key(KeyInput {
        key: Some(crate::keys::Key::Enter),
        text: String::new(),
        mods: Default::default(),
    }));
    let saved = crate::config::preferences::read(&preferences.path).unwrap();
    assert_eq!(saved["translation"]["provider"], "plugin:bilibili");
    assert_eq!(saved["translation"]["openai"]["model"], "custom-model");
    assert_eq!(saved["translation"]["bilibili"]["model"], "keep-model");
    preferences.event(WindowEvent::Resize { size: (580, 420) });
    let layout = Layout::new(&preferences);
    assert!(preferences.translation.scroll <= layout.maximum);
    assert!(
        layout
            .controls
            .iter()
            .any(|(hit, _)| *hit == Hit::Field(Field::Provider))
    );
    assert!(
        layout
            .hit((
                layout.viewport.left() as f64 + 10.0,
                layout.viewport.bottom() as f64 + 1.0
            ))
            .is_none()
    );
}
