//! 【系统托盘】【菜单模型】沿用 Qt 菜单顺序、延时子菜单和启用状态，平台只负责展示与回调

use super::super::Action;
use crate::ui::language::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::features::desktop) struct State {
    pub language: Language,
    pub delayed: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Item {
    Action {
        label: String,
        action: Action,
        enabled: bool,
    },
    Submenu {
        label: String,
        items: Vec<Item>,
    },
    Separator,
}

/// 【系统托盘】【菜单构造】按 state 返回截图与录制菜单，取消延时项仅在有待执行计划时可用
pub(super) fn entries(state: State) -> Vec<Item> {
    let chinese = state.language == Language::Chinese;
    let label = |cn: &str, en: &str| if chinese { cn } else { en }.to_owned();
    let action = |cn: &str, en: &str, action| Item::Action {
        label: label(cn, en),
        action,
        enabled: true,
    };
    let mut delays: Vec<_> = [3, 5, 10]
        .into_iter()
        .map(|seconds| Item::Action {
            label: if chinese {
                format!("{seconds} 秒后")
            } else {
                format!("After {seconds} seconds")
            },
            action: Action::Delay(seconds),
            enabled: true,
        })
        .collect();
    delays.push(Item::Separator);
    delays.push(Item::Action {
        label: label("取消延时截图", "Cancel Delayed Capture"),
        action: Action::CancelDelay,
        enabled: state.delayed,
    });
    vec![
        action("截图", "Capture", Action::Capture),
        action("全屏截图", "Fullscreen Capture", Action::Fullscreen),
        action("剪贴板贴图", "Pin Clipboard", Action::Pin),
        Item::Submenu {
            label: label("延时截图", "Delayed Capture"),
            items: delays,
        },
        Item::Submenu {
            label: label("屏幕录制", "Recording"),
            items: vec![
                action("开始录制", "Start Recording", Action::Record),
                action("暂停录制", "Pause Recording", Action::PauseRecording),
                action("继续录制", "Resume Recording", Action::ResumeRecording),
                action("停止并保存", "Stop and Save", Action::StopRecording),
            ],
        },
        action("设置", "Settings", Action::Settings),
        action("截图历史", "Screenshot History", Action::History),
        Item::Separator,
        action("退出", "Quit", Action::Quit),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【系统托盘测试】【Qt 菜单对齐】无参数，中英文菜单顺序与延时状态一致，录制命令独立分组
    #[test]
    fn qt_menu_order_and_delay_state_are_shared_by_platforms() {
        for language in [Language::Chinese, Language::English] {
            for delayed in [false, true] {
                let menu = entries(State { language, delayed });
                let actions: Vec<_> = menu
                    .iter()
                    .filter_map(|item| match item {
                        Item::Action {
                            action,
                            enabled: true,
                            ..
                        } => Some(*action),
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    actions,
                    [
                        Action::Capture,
                        Action::Fullscreen,
                        Action::Pin,
                        Action::Settings,
                        Action::History,
                        Action::Quit
                    ]
                );
                assert_eq!(menu.len(), 9);
                assert_eq!(menu[7], Item::Separator);
                let Item::Submenu { items, .. } = &menu[3] else {
                    panic!("missing delay submenu")
                };
                assert_eq!(items.len(), 5);
                for (item, seconds) in items.iter().zip([3, 5, 10]) {
                    assert!(
                        matches!(item, Item::Action { action: Action::Delay(value), enabled: true, .. } if *value == seconds)
                    );
                }
                assert_eq!(items[3], Item::Separator);
                assert!(
                    matches!(items[4], Item::Action { action: Action::CancelDelay, enabled, .. } if enabled == delayed)
                );
            }
        }
    }
}
