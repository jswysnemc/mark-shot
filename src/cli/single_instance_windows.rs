//! 【Windows启动】【截图占用】编辑截图持有原生文件锁，独立图片与快捷捕获保持各自生命周期

/// 【Windows启动】【会话执行】按connection和launch取得普通截图占用后运行共享编辑器，返回完整会话结果
pub(super) async fn run(
    connection: crate::backend::session::Session,
    launch: crate::app::Launch,
) -> crate::AppResult {
    if launch.image.is_some()
        || launch.mode != crate::types::Mode::Editor
        || !launch.display_names.is_empty()
    {
        return crate::app::run(connection, launch).await;
    }
    let owner = crate::features::single_instance::Owner::start(
        crate::features::single_instance::request::Plan {
            fullscreen: launch.fullscreen,
            all_outputs: !crate::config::preferences::current().capture.one_monitor(
                launch.one_monitor,
                launch.all_outputs,
                launch.fullscreen,
            ),
            deadline: launch.shot_at,
        },
    )?;
    let _scope = crate::features::single_instance::context::enter(&owner.controller);
    crate::app::run(connection, launch).await
}
