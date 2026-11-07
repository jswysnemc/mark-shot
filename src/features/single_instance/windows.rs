//! 【Windows单实例】【截图占用】原生文件锁覆盖当前截图，系统在进程退出后自动释放

#[path = "context.rs"]
pub(crate) mod context;
pub(crate) mod request {
    #[derive(Clone, Copy)]
    pub struct Plan {
        pub fullscreen: bool,
        pub all_outputs: bool,
        pub deadline: Option<std::time::Instant>,
    }
}
use request::Plan;
use std::{sync::Arc, time::Instant};

pub(crate) struct Controller {
    plan: Plan,
}
impl Controller {
    /// 【Windows单实例】【首帧确认】无参数，当前平台仅保持截图占用，不安排重复捕获，无返回值
    pub fn active(&self) {}
    /// 【Windows单实例】【开始确认】无参数，到期时返回本次计划，未来期限返回空值
    pub fn begin(&self) -> Option<Plan> {
        self.plan
            .deadline
            .is_none_or(|at| at <= Instant::now())
            .then_some(self.plan)
    }
    /// 【Windows单实例】【等待计划】无参数，返回当前计划供现有延迟逻辑等待
    pub fn pending(&self) -> Option<Plan> {
        Some(self.plan)
    }
    /// 【Windows单实例】【变更等待】无参数，当前无跨进程请求入口，保持等待直到调用范围取消，无返回值
    pub async fn changed(&self) {
        std::future::pending::<()>().await;
    }
}

pub(crate) struct Owner {
    pub controller: Arc<Controller>,
    _lock: crate::utils::fs_lock::Lock,
}
impl Owner {
    /// 【Windows单实例】【范围占用】按plan占用当前配置的截图范围，返回自动释放锁，重复启动明确返回已有截图错误
    pub fn start(plan: Plan) -> crate::AppResult<Self> {
        let root = dirs::cache_dir()
            .ok_or("Windows cache directory is unavailable")?
            .join("mark-shot");
        std::fs::create_dir_all(&root)?;
        let file = crate::utils::platform::open_regular(&root.join("capture.instance.lock"), true)?;
        let lock =
            crate::utils::fs_lock::acquire(file, std::time::Duration::ZERO).map_err(|error| {
                format!("a capture instance is already active or its lock is unavailable: {error}")
            })?;
        Ok(Self {
            controller: Arc::new(Controller { plan }),
            _lock: lock,
        })
    }
}
