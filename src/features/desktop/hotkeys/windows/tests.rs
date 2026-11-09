//! 【Windows快捷键测试】【真实注册】覆盖系统占用与取消后的注册资源回收

use super::*;
use crate::features::desktop::{Event, status::Registration};

/// 【Windows快捷键测试】【键盘布局】无参数，注册标点时保留系统映射所需修饰键，扩展Unicode不能截断成其他按键
#[test]
fn character_mapping_preserves_required_layout_modifiers() {
    for character in ['?', '+', '@'] {
        let mapped = unsafe { VkKeyScanW(character as u16) };
        if mapped == -1 {
            continue;
        }
        let (key, modifiers) = virtual_key(Key::Char(character)).unwrap();
        assert_eq!(key, u32::from(mapped as u16 & 255));
        for (bit, flag) in [(1, MOD_SHIFT), (2, MOD_CONTROL), (4, MOD_ALT)] {
            assert_eq!(modifiers & flag != 0, (mapped as u16 >> 8) & bit != 0);
        }
    }
    assert!(virtual_key(Key::Char('\u{1003f}')).is_err());
}

/// 【Windows快捷键测试】【占用回退】无参数，模拟系统占用PrintScreen，要求后台仍建立截图监听且取消可退出
#[test]
fn occupied_print_screen_still_registers_and_releases() {
    let (ready, started) = std::sync::mpsc::channel();
    let (stop, stopped) = std::sync::mpsc::channel();
    let owner = std::thread::spawn(move || unsafe {
        let registered =
            RegisterHotKey(std::ptr::null_mut(), 71, MOD_NOREPEAT, VK_SNAPSHOT.into()) != 0;
        ready.send(registered).unwrap();
        stopped.recv().unwrap();
        if registered {
            UnregisterHotKey(std::ptr::null_mut(), 71);
        }
    });
    let _owned = started.recv().unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (result, activated, repeated) = runtime.block_on(async {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let reporter = Reporter {
            sender,
            generation: 1,
            hotkey: true,
        };
        let (cancel, cancelled) = tokio::sync::watch::channel(false);
        let task = tokio::spawn(super::super::run("PrintScreen".into(), reporter, cancelled));
        let result = tokio::time::timeout(std::time::Duration::from_secs(3), receiver.recv()).await;
        let mut activated = false;
        let mut repeated = false;
        if matches!(
            result,
            Ok(Some(Event::Status(_, true, Registration::Registered(_))))
        ) {
            let events = [0, 0, KEYEVENTF_KEYUP].map(|flags| INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_SNAPSHOT,
                        dwFlags: flags,
                        ..unsafe { std::mem::zeroed() }
                    },
                },
            });
            assert_eq!(
                unsafe {
                    SendInput(
                        events.len() as u32,
                        events.as_ptr(),
                        size_of::<INPUT>() as i32,
                    )
                },
                3
            );
            activated = matches!(
                tokio::time::timeout(std::time::Duration::from_secs(3), receiver.recv()).await,
                Ok(Some(Event::Action(Action::Capture, _)))
            );
            repeated = tokio::time::timeout(std::time::Duration::from_millis(80), receiver.recv())
                .await
                .is_ok();
        }
        cancel.send(true).ok();
        tokio::time::timeout(std::time::Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap();
        (result, activated, repeated)
    });
    stop.send(()).unwrap();
    owner.join().unwrap();
    assert!(
        matches!(
            result,
            Ok(Some(Event::Status(_, true, Registration::Registered(_))))
        ),
        "PrintScreen collision must use its dedicated hook fallback"
    );
    assert!(
        activated,
        "PrintScreen did not deliver the screenshot action"
    );
    assert!(
        !repeated,
        "key repeat delivered more than one screenshot action"
    );
}
