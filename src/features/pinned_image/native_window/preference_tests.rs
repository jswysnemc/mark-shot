//! 【贴图菜单测试】【偏好持久化】真实菜单动作保存后重新创建窗口，失败不修改当前选项

use super::*;
use crate::config::pinned_preferences;

/// 【贴图菜单测试】【内容创建】按 preferences 创建无原生窗口的测试内容，返回具有置顶能力的贴图状态
fn pin(preferences: &crate::config::Pinned) -> Pin {
    let png = crate::utils::encode_rgba(&[71, 92, 113, 255], 1, 1);
    let mut pin = Pin::new(
        PinnedImage::decode(&png, 1.0).unwrap(),
        None,
        None,
        preferences,
    )
    .unwrap();
    pin.top_supported = true;
    // 1. 【贴图菜单测试】【任务隔离】让识别入口复用等待状态，测试菜单开关不启动识别或弹出窗口
    pin.model_waiting = true;
    pin
}

/// 【贴图菜单测试】【开关往返】无参数，菜单写入 TOML 与旧 JSON 后新窗口恢复两个选项，保留其他配置，无返回值
#[test]
fn menu_toggles_persist_for_new_pin_windows_in_both_config_formats() {
    for extension in ["toml", "json"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(format!("config.{extension}"));
        std::fs::write(&path, if extension == "toml" {
            "# keep\n[pinned]\nalways_on_top=true\nauto_text_selection=false\nborder_width=3.0\n[custom]\nvalue=7\n"
        } else {
            r#"{"pinnedWindow":{"alwaysOnTop":true,"autoTextSelection":false,"border":{"width":3.0}},"custom":{"value":7}}"#
        }).unwrap();
        let initial = crate::config::preferences::load(&path).unwrap().pinned;
        let mut current = pin(&initial);
        for action in [PinAction::AlwaysOnTop, PinAction::TextSelection] {
            current
                .action_with(action, |preference, enabled| {
                    pinned_preferences::save_at(&path, preference, enabled)
                })
                .unwrap();
        }
        let restored = crate::config::preferences::load(&path).unwrap().pinned;
        assert!(!restored.always_on_top, "menu did not save always_on_top");
        assert!(
            restored.auto_text_selection,
            "menu did not save auto_text_selection"
        );
        assert_eq!(restored.border_width, 3.0);
        let next = pin(&restored);
        assert!(!next.image.always_on_top);
        assert!(next.text_selection);
        assert_eq!(
            crate::config::settings_store::source(&path).unwrap()["custom"]["value"],
            7
        );
        // 1. 【贴图菜单测试】【反向保存】关闭划词和重新置顶同样保存，不能只支持首次启用
        let mut next = next;
        for action in [PinAction::AlwaysOnTop, PinAction::TextSelection] {
            next.action_with(action, |preference, enabled| {
                pinned_preferences::save_at(&path, preference, enabled)
            })
            .unwrap();
        }
        let restored = crate::config::preferences::load(&path).unwrap().pinned;
        assert!(restored.always_on_top);
        assert!(!restored.auto_text_selection);
    }
}

/// 【贴图菜单测试】【保存失败】无参数，模拟写入失败，菜单返回错误且当前开关保持原值，无返回值
#[test]
fn failed_preference_write_does_not_change_the_current_menu_toggle() {
    for action in [PinAction::AlwaysOnTop, PinAction::TextSelection] {
        let mut current = pin(&crate::config::Pinned::default());
        let top = current.image.always_on_top;
        let selection = current.text_selection;
        let result = current.action_with(action, |_, _| {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "test save denied",
            ))
        });
        assert!(result.is_err(), "menu did not attempt persistence");
        assert_eq!(current.image.always_on_top, top);
        assert_eq!(current.text_selection, selection);
    }
}
