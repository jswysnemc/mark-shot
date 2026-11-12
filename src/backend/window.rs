//! 【原生窗口】【共用契约】窗口内容与原生生命周期之间的统一事件接口

mod scroll;
#[cfg(target_os = "linux")]
pub use super::wayland::window::WindowWake;

/// 【Linux窗口】【运行分发】按spec和content启动当前协议的原生窗口，返回生命周期结果
#[cfg(target_os = "linux")]
pub fn run(spec: WindowSpec, content: impl WindowContent + 'static) -> crate::AppResult {
    match super::session::Session::connect_to_env()? {
        super::session::Session::Wayland(_) => super::wayland::window::run(spec, content),
        super::session::Session::X11(display) => super::x11::window::run(display, spec, content),
    }
}
#[cfg(target_os = "windows")]
pub use super::windows::window::{WindowWake, run};
pub use scroll::{ScrollInput, ScrollSource};

// 1. 【原生窗口】【首帧期限】冷启动包含系统装载与界面准备，Windows给予有界余量，收到实际首帧确认后立即交接
#[cfg(target_os = "linux")]
pub const READY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
#[cfg(target_os = "windows")]
pub const READY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

use crate::keys::{Key, Mods};
use crate::types::popup::{PopupEvent, PopupFrame};
use crate::types::text_input::{ChangeCause, Edit, Focus, Snapshot};
use tiny_skia::Pixmap;

pub struct WindowDialog {
    pub id: u64,
    pub spec: WindowSpec,
    pub content: Box<dyn WindowContent>,
}

pub const LEFT_BUTTON: u32 = 0x110;
pub const RIGHT_BUTTON: u32 = 0x111;

#[derive(Clone)]
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
    Wheel(ScrollInput),
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
                | Self::Wheel(_)
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
    ResizeEdge(WindowEdge),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEdge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum WindowCursor {
    #[default]
    Default,
    Text,
    Pointer,
    Grab,
    Grabbing,
    Resize,
    HorizontalResize,
    VerticalResize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WindowPopupAction {
    #[default]
    None,
    Redraw,
    Close,
    Activate(usize),
    CloseAll,
}

pub trait WindowContent {
    /// 【原生窗口】【关闭确认】无参数，默认允许关闭；有编辑草稿的内容可提交失败后保留窗口
    fn close_requested(&mut self) -> bool {
        true
    }
    /// 【原生窗口】【初始位置】无参数，返回内容要求的全局位置，默认由平台居中布置
    fn initial_position(&self) -> Option<(i32, i32)> {
        None
    }

    /// 【原生窗口】【内容尺寸】无参数，返回内容主动请求的逻辑尺寸，默认保留用户窗口尺寸
    fn preferred_size(&self) -> Option<(u32, u32)> {
        None
    }

    /// 【原生窗口】【一次性位移】取出内容请求的逻辑位移，默认不移动；原生后端按当前DPI与现有窗口位置应用
    fn take_position_offset(&mut self) -> Option<(f64, f64)> {
        None
    }

    /// 【原生窗口】【置顶请求】返回内容当前要求的主窗口置顶状态，默认普通窗口，无参数
    fn always_on_top(&self) -> bool {
        false
    }

    /// 【原生窗口】【浮层优先级】无参数，返回置顶窗口是否高于普通贴图；默认 false，Wayland 映射为 Overlay 层
    fn above_pinned_windows(&self) -> bool {
        false
    }

    /// 【原生窗口】【置顶能力】通知 supported 是否可使用真实置顶角色，内容不得把不支持的请求显示为已成功，无返回值
    fn stacking_supported(&mut self, _supported: bool) {}
    /// 【原生窗口】【内容期限】返回下一次单次唤醒的绝对期限，无参数；到期 Wake 后须消费或更新，默认无期限及周期工作
    fn next_deadline(&self) -> Option<std::time::Instant> {
        None
    }

    /// 【原生窗口】【弹层身份】返回待显示或存活菜单的稳定编号；默认没有菜单，无参数
    fn popup_id(&self) -> Option<u64> {
        None
    }

    /// 【原生窗口】【弹层链】返回根菜单及逐级子菜单的定位节点，无参数；默认保持原有单层菜单接口
    fn popup_nodes(&self) -> Vec<crate::types::popup_tree::PopupNode> {
        self.popup_id()
            .map(crate::types::popup_tree::PopupNode::root)
            .into_iter()
            .collect()
    }

    /// 【原生窗口】【指定弹层帧】按 id 和 scale 返回变化后的像素，默认只支持根菜单，不复制静止缓冲
    fn popup_frame_for(&mut self, id: u64, scale: f32) -> Option<PopupFrame<'_>> {
        if self.popup_id() == Some(id) {
            self.popup_frame(scale)
        } else {
            None
        }
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

    /// 【原生窗口】【输入停用】临时切换目标时暂停候选交互，返回是否重绘；默认清理候选，内容可独立保留只读显示，无参数
    fn suspend_text_input(&mut self) -> bool {
        self.cancel_text_input()
    }

    /// 【原生窗口】【唤醒接入】接收 wake 以通知后台任务完成；默认不启动后台工作
    fn connected(&mut self, _wake: WindowWake) {}

    /// 【原生窗口】【内容绘制】按 scale 将当前内容绘制到 canvas；不创建窗口或提交协议请求
    fn draw(&mut self, canvas: &mut Pixmap, scale: f32);

    /// 【原生窗口】【内容事件】处理 event；返回重绘、关闭或窗口管理请求
    fn event(&mut self, event: WindowEvent) -> WindowAction;

    /// 【原生窗口】【完整滚动】处理 input 的协议来源与刻度，默认适配为既有 Scroll 事件，避免改变其他内容窗口的像素语义
    fn scroll(&mut self, input: ScrollInput) -> WindowAction {
        self.event(WindowEvent::Scroll {
            position: input.position,
            delta: input.delta,
        })
    }

    /// 【原生窗口】【首帧通知】首帧提交后通知内容；默认不执行操作
    fn mapped(&mut self) {}

    /// 【原生窗口】【鼠标形状】根据 position 返回鼠标形状；默认使用普通指针
    fn cursor(&self, _position: (f64, f64)) -> WindowCursor {
        WindowCursor::Default
    }
}
