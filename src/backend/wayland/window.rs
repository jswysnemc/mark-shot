//! 【原生窗口】【接口】普通 xdg-toplevel 窗口与内容之间的事件契约

mod dialog;
mod dialog_protocol;
mod input;
mod popup;
mod popup_input;
mod protocol;
mod runtime;
mod shell;
mod surface;
mod text_input;
mod wake;

use tiny_skia::Pixmap;

use crate::keys::{Key, Mods};
use crate::types::popup::{PopupEvent, PopupFrame};
use crate::types::text_input::{ChangeCause, Edit, Focus, Snapshot};

pub use dialog::WindowDialog;
pub use runtime::run;
pub use wake::WindowWake;

pub const LEFT_BUTTON: u32 = 0x110;
pub const RIGHT_BUTTON: u32 = 0x111;

pub struct WindowSpec {
    pub title: String,
    pub app_id: String,
    pub size: (u32, u32),
    pub minimum: (u32, u32),
}

#[derive(Debug, Clone)]
pub struct KeyInput {
    pub key: Option<Key>,
    pub text: String,
    pub mods: Mods,
}

#[derive(Debug, Clone)]
pub enum WindowEvent {
    Wake,
    Resize {
        size: (u32, u32),
    },
    Focused(bool),
    Key(KeyInput),
    KeyRelease(KeyInput),
    TextInput(Edit),
    PointerMove {
        position: (f64, f64),
    },
    PointerButton {
        position: (f64, f64),
        button: u32,
        pressed: bool,
    },
    Scroll {
        position: (f64, f64),
        delta: (f64, f64),
    },
    PointerLeave,
}

impl WindowEvent {
    /// 【原生窗口】【交互分类】返回本事件是否为模态时需隔离的用户输入；配置、焦点与后台唤醒不是交互，无参数
    pub fn is_interaction(&self) -> bool {
        matches!(
            self,
            Self::Key(_)
                | Self::KeyRelease(_)
                | Self::TextInput(_)
                | Self::PointerMove { .. }
                | Self::PointerButton { .. }
                | Self::Scroll { .. }
                | Self::PointerLeave
        )
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WindowAction {
    #[default]
    None,
    Redraw,
    Close,
    Move,
    Resize,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum WindowCursor {
    #[default]
    Default,
    Text,
    Pointer,
    Resize,
    HorizontalResize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WindowPopupAction {
    #[default]
    None,
    Redraw,
    Close,
    Activate(usize),
}

pub trait WindowContent {
    /// 【原生窗口】【弹层身份】返回待显示或存活菜单的稳定编号；默认没有菜单，无参数
    fn popup_id(&self) -> Option<u64> {
        None
    }

    /// 【原生窗口】【弹层像素】按父窗口 scale 返回变化后的菜单帧；静止菜单返回空值，关闭通过 popup_id 表达
    fn popup_frame(&mut self, _scale: f32) -> Option<PopupFrame<'_>> {
        None
    }

    /// 【原生窗口】【弹层输入】接收 id 对应 event，返回重绘或待交付动作；此阶段不得执行业务修改
    fn popup_event(&mut self, _id: u64, _event: PopupEvent) -> WindowPopupAction {
        WindowPopupAction::None
    }

    /// 【原生窗口】【弹层交付】原生表面销毁后交付 id 的 action，空值表示取消；返回父窗口动作
    fn popup_closed(&mut self, _id: u64, _action: Option<usize>) -> WindowAction {
        WindowAction::None
    }

    /// 【原生窗口】【对话框请求】取出一个待显示的独立模态子窗口；默认没有请求，无参数
    fn take_dialog(&mut self) -> Option<WindowDialog> {
        None
    }

    /// 【原生窗口】【对话框结果】接收 id 对应子窗口的 accepted 结果；默认不执行业务，无其他返回信息
    fn dialog_closed(&mut self, _id: u64, _accepted: bool) -> WindowAction {
        WindowAction::None
    }

    /// 【原生窗口】【确认结果】内容请求关闭时返回是否明确确认；窗口管理器关闭不使用此值，无参数
    fn dialog_accepted(&self) -> bool {
        false
    }

    /// 【原生窗口】【输入目标】返回当前可编辑字段的稳定身份，菜单、原图或只读内容返回空值，无参数
    fn text_input_target(&self) -> Option<u64> {
        None
    }

    /// 【原生窗口】【输入快照】用 focus、target_id、previous 和 cause 返回变化后的有界快照；空值表示没有变化，不分配静止帧文字
    fn text_input_snapshot(
        &mut self,
        _focus: Focus,
        _target_id: u64,
        _previous: Option<&Snapshot>,
        _cause: ChangeCause,
    ) -> Option<Snapshot> {
        None
    }

    /// 【原生窗口】【候选取消】取消显示候选但不回退已确认正文，返回是否需要重绘，无参数
    fn cancel_text_input(&mut self) -> bool {
        false
    }

    /// 【原生窗口】【唤醒接入】接收 wake 以通知后台任务完成；默认不启动后台工作
    fn connected(&mut self, _wake: WindowWake) {}

    /// 【原生窗口】【内容绘制】按 scale 将当前内容绘制到 canvas；不创建窗口或提交协议请求
    fn draw(&mut self, canvas: &mut Pixmap, scale: f32);

    /// 【原生窗口】【内容事件】处理 event；返回重绘、关闭或窗口管理请求
    fn event(&mut self, event: WindowEvent) -> WindowAction;

    /// 【原生窗口】【首帧通知】首帧提交后通知内容；默认不执行操作
    fn mapped(&mut self) {}

    /// 【原生窗口】【鼠标形状】根据 position 返回鼠标形状；默认使用普通指针
    fn cursor(&self, _position: (f64, f64)) -> WindowCursor {
        WindowCursor::Default
    }
}
