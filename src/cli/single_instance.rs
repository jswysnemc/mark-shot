//! 【单实例】【命令入口】普通截图及全屏编辑进入同一占用范围，独立图片与快捷捕获保持自身入口

use crate::{
    app::Launch,
    features::single_instance::{
        Entry, Owner, context,
        request::{Plan, Request},
    },
    types::Mode,
};
use std::time::Instant;

/// 【单实例】【截图执行】按 connection 与 launch 占用或转交普通截图命令，返回截图结果；已有实例确认后不创建覆盖窗口
pub(super) async fn run(
    connection: crate::backend::session::Session,
    launch: Launch,
) -> crate::AppResult {
    if launch.image.is_some() || launch.mode != Mode::Editor || !launch.display_names.is_empty() {
        return crate::app::run(connection, launch).await;
    }
    let now = Instant::now();
    let request = Request {
        capture: true,
        fullscreen: launch.fullscreen,
        all_outputs: !launch.one_monitor,
        capture_delay_ms: launch.shot_at.map_or(0, |at| {
            i64::try_from(at.saturating_duration_since(now).as_millis()).unwrap_or(i64::MAX)
        }),
        ..Request::default()
    };
    let plan = Plan {
        fullscreen: launch.fullscreen,
        all_outputs: !launch.one_monitor,
        deadline: launch.shot_at,
    };
    let owner = match Owner::start(&connection, request, plan).await? {
        Entry::Forwarded => {
            log::info!("【单实例】【命令转交】截图请求已由现有实例确认");
            return Ok(());
        }
        Entry::Owner(owner) => owner,
    };
    let _scope = context::enter(&owner.controller);
    let controller = owner.controller.clone();
    // 1. 【单实例】【准备阶段】异步捕获等待时响应命令，进入原生窗口后由该窗口等待同一描述符
    tokio::select! {
        biased;
        result = crate::app::run(connection, launch) => result,
        result = controller.serve() => match result {
            Err(error) => Err(error),
            Ok(()) => Err("single-instance service stopped before capture finished".into()),
        },
    }
}
