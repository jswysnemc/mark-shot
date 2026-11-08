//! 【设置回归】【真实控件】覆盖原生窗口事件与业务配置，不以写入文件代替交互验证

use super::{catalog::Page, window::Preferences};
use crate::backend::window::{WindowContent, WindowEvent};

/// 【设置回归】【滚动条命中】无参数，窄窗口内容溢出时右侧轨道必须可操作，拖到底部能访问末项
#[test]
fn overflow_has_a_draggable_scrollbar() {
    let mut state = Preferences::new(false).unwrap();
    state.page = Page::Capture;
    state.event(WindowEvent::Resize { size: (580, 420) });
    assert!(
        state.hit((566.0, 153.0)).is_some(),
        "overflow has no scrollbar hit target"
    );
    state.event(WindowEvent::PointerButton {
        position: (566.0, 153.0),
        button: 0x110,
        pressed: true,
    });
    state.event(WindowEvent::PointerMove {
        position: (566.0, 350.0),
    });
    state.event(WindowEvent::PointerButton {
        position: (566.0, 350.0),
        button: 0x110,
        pressed: false,
    });
    assert_eq!(state.scroll, state.visible().len() - state.capacity());
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
    assert_eq!(state.scroll, 1);
    state.event(WindowEvent::Key(KeyInput {
        key: Some(Key::End),
        text: String::new(),
        mods: Default::default(),
    }));
    assert_eq!(state.scroll, state.visible().len() - state.capacity());
    state.event(WindowEvent::Resize { size: (900, 1200) });
    assert!(state.scrollbar().is_none());
    assert_eq!(state.scroll, 0);
}
