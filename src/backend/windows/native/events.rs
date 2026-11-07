//! 【Windows输入】【有序事件】原生回调仅收集有序事件，业务在拥有窗口的线程中消费

use std::{cell::RefCell, collections::VecDeque, rc::Rc};

#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub super_key: bool,
}

#[derive(Clone, Debug)]
pub enum Event {
    Close {
        id: u64,
    },
    Resize {
        id: u64,
        size: (u32, u32),
    },
    Dpi {
        id: u64,
        dpi: u32,
        bounds: super::Bounds,
    },
    Focus {
        id: u64,
        focused: bool,
    },
    Pointer {
        id: u64,
        position: (f64, f64),
    },
    Leave {
        id: u64,
    },
    Button {
        id: u64,
        position: (f64, f64),
        button: u32,
        pressed: bool,
        time: u32,
    },
    Wheel {
        id: u64,
        position: (f64, f64),
        amount: i16,
        horizontal: bool,
        modifiers: Modifiers,
    },
    Key {
        id: u64,
        code: u32,
        pressed: bool,
        repeated: bool,
        modifiers: Modifiers,
    },
    Composition {
        id: u64,
        target: u64,
        preedit: String,
        cursor: usize,
        commit: String,
    },
    Character {
        id: u64,
        character: u16,
    },
}

impl Event {
    /// 【Windows输入】【窗口身份】无参数，返回事件创建时绑定的窗口编号，过期消息不会匹配新建窗口
    pub fn id(&self) -> u64 {
        match self {
            Self::Close { id }
            | Self::Resize { id, .. }
            | Self::Dpi { id, .. }
            | Self::Focus { id, .. }
            | Self::Pointer { id, .. }
            | Self::Leave { id }
            | Self::Button { id, .. }
            | Self::Wheel { id, .. }
            | Self::Key { id, .. }
            | Self::Composition { id, .. }
            | Self::Character { id, .. } => *id,
        }
    }
}

#[derive(Clone, Default)]
pub struct EventQueue(Rc<RefCell<VecDeque<Event>>>);

impl EventQueue {
    /// 【Windows输入】【事件取出】无参数，返回最早事件，没有事件时不分配，不跨线程持有窗口状态
    pub fn pop(&self) -> Option<Event> {
        self.0.borrow_mut().pop_front()
    }

    /// 【Windows输入】【回调收集】将event追加到当前线程队列，无返回值，不在回调中执行编辑或渲染业务
    pub(super) fn push(&self, event: Event) {
        self.0.borrow_mut().push_back(event);
    }
}

/// 【Windows输入】【修饰键快照】无参数，返回当前系统修饰键，仅在窗口线程接收实际输入时调用
pub(super) fn modifiers() -> Modifiers {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
    };
    let held = |key| unsafe { GetKeyState(key as i32) } < 0;
    Modifiers {
        ctrl: held(VK_CONTROL),
        shift: held(VK_SHIFT),
        alt: held(VK_MENU),
        super_key: held(VK_LWIN) || held(VK_RWIN),
    }
}
