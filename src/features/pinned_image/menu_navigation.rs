//! 【贴图菜单】【键盘导航】按可用能力选择菜单行，分隔线和禁用动作不能成为键盘确认目标

use super::menu::{ITEMS, PinAction};
use crate::keys::Key;

/// 【贴图菜单】【可用行】按index和enabled检查菜单项，返回是否允许确认，越界和分隔线均拒绝
pub fn available(index: usize, enabled: impl Fn(PinAction) -> bool) -> bool {
    ITEMS
        .get(index)
        .and_then(|item| item.action)
        .is_some_and(enabled)
}

/// 【贴图菜单】【移动选择】按current、key和enabled返回新的可用行，首尾键定位边界，方向键循环；其他键保持原选择
pub fn navigate(
    current: Option<usize>,
    key: Key,
    enabled: impl Fn(PinAction) -> bool,
) -> Option<usize> {
    let count = ITEMS.len();
    let current = current.filter(|index| *index < count);
    match key {
        Key::Home => (0..count).find(|index| available(*index, &enabled)),
        Key::End => (0..count).rev().find(|index| available(*index, &enabled)),
        Key::Down | Key::Up => {
            let forward = key == Key::Down;
            let start = current.unwrap_or(if forward { count - 1 } else { 0 });
            (1..=count)
                .map(|step| {
                    if forward {
                        (start + step) % count
                    } else {
                        (start + count - step) % count
                    }
                })
                .find(|index| available(*index, &enabled))
        }
        _ => current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【贴图菜单测试】【循环与边界】无参数，确认方向键循环且跳过分隔线及禁用项，无返回值
    #[test]
    fn navigation_skips_disabled_rows_and_wraps() {
        let enabled = |action| {
            matches!(
                action,
                PinAction::RotateLeft | PinAction::ZoomIn | PinAction::Close
            )
        };
        assert_eq!(navigate(None, Key::Down, enabled), Some(0));
        assert_eq!(navigate(None, Key::Up, enabled), Some(19));
        assert_eq!(navigate(Some(0), Key::Down, enabled), Some(3));
        assert_eq!(navigate(Some(3), Key::Up, enabled), Some(0));
        assert_eq!(navigate(Some(19), Key::Down, enabled), Some(0));
        assert_eq!(navigate(Some(0), Key::Up, enabled), Some(19));
        assert_eq!(navigate(Some(3), Key::Home, enabled), Some(0));
        assert_eq!(navigate(Some(3), Key::End, enabled), Some(19));
        assert!(!available(1, enabled));
        assert!(!available(2, enabled));
        assert!(!available(usize::MAX, enabled));
    }

    /// 【贴图菜单测试】【空能力】无参数，没有可用动作时不返回禁用行，过期索引不会溢出，无返回值
    #[test]
    fn empty_capabilities_and_stale_selection_are_safe() {
        for key in [Key::Home, Key::End, Key::Up, Key::Down] {
            assert_eq!(navigate(Some(usize::MAX), key, |_| false), None);
        }
        assert_eq!(navigate(Some(usize::MAX), Key::Down, |_| true), Some(0));
        assert_eq!(navigate(Some(3), Key::Left, |_| true), Some(3));
    }
}
