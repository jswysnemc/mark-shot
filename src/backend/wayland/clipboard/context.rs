//! 【Wayland剪贴板】【窗口上下文】复制操作复用当前窗口连接，禁止在沙箱中转交无焦点的 X11 进程

use std::cell::RefCell;

pub(crate) enum Content {
    Image(Vec<u8>),
    Text(String),
}

type Copy = Box<dyn FnMut(Content) -> crate::AppResult>;
thread_local! {
    static CURRENT: RefCell<Option<Copy>> = const { RefCell::new(None) };
}

pub(crate) struct Scope(Option<Copy>);

/// 【Wayland剪贴板】【上下文登记】按 copy 登记窗口复制处理器，返回恢复句柄；原生数据控制会话不调用此方法
pub(crate) fn enter(copy: impl FnMut(Content) -> crate::AppResult + 'static) -> Scope {
    Scope(CURRENT.with(|current| current.replace(Some(Box::new(copy)))))
}

/// 【Wayland剪贴板】【图像复制】按 png 尝试当前窗口处理器，返回未启用或复制结果，不启动辅助进程
pub(crate) fn image(png: &[u8]) -> Option<crate::AppResult> {
    CURRENT.with(|current| {
        current
            .borrow_mut()
            .as_mut()
            .map(|copy| copy(Content::Image(png.to_vec())))
    })
}

/// 【Wayland剪贴板】【文字复制】按 text 尝试当前窗口处理器，返回未启用或复制结果，不修改主选择区
pub(crate) fn text(text: &str) -> Option<crate::AppResult> {
    CURRENT.with(|current| {
        current
            .borrow_mut()
            .as_mut()
            .map(|copy| copy(Content::Text(text.to_owned())))
    })
}

impl Drop for Scope {
    /// 【Wayland剪贴板】【上下文恢复】无参数，恢复进入前的窗口处理器，无返回值，不影响其他线程
    fn drop(&mut self) {
        CURRENT.with(|current| current.replace(self.0.take()));
    }
}
