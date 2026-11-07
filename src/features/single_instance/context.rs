//! 【单实例】【运行范围】当前线程创建的截图与辅助窗口共用控制句柄，独立进程不继承占用

use super::Controller;
use std::{
    cell::RefCell,
    sync::{Arc, Weak},
};

thread_local! {
    static CURRENT: RefCell<Weak<Controller>> = const { RefCell::new(Weak::new()) };
}

pub struct Scope {
    previous: Weak<Controller>,
}

/// 【单实例】【进入范围】按 controller 注册本次截图上下文，返回自动恢复句柄，不延长套接字占用生命周期
pub fn enter(controller: &Arc<Controller>) -> Scope {
    Scope {
        previous: CURRENT.with(|current| current.replace(Arc::downgrade(controller))),
    }
}

/// 【单实例】【当前句柄】无参数，返回当前截图控制器；历史、贴图和独立图片窗口没有上下文时返回空值
pub fn current() -> Option<Arc<Controller>> {
    CURRENT.with(|current| current.borrow().upgrade())
}

impl Drop for Scope {
    /// 【单实例】【恢复范围】恢复进入之前的线程上下文，无参数、无返回值，不影响其他线程或独立窗口进程
    fn drop(&mut self) {
        let previous = std::mem::take(&mut self.previous);
        CURRENT.with(|current| current.replace(previous));
    }
}
