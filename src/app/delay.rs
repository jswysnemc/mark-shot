// --delay: wait until the shot time, letting the capture backend prepare meanwhile,
// with a countdown when run from a terminal
use crate::backend::CaptureMethod;
use crate::profiler::Profiler;

use std::io::IsTerminal;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(10);

/// 【截图延迟】【原生调度】按 launch、capture 与 prof 准备截图并等待最新单实例计划，返回准备或占用错误，不创建轮询线程
pub(super) async fn wait_launch(
    launch: &mut super::Launch,
    capture: &dyn CaptureMethod,
    prof: &mut Profiler,
) -> crate::AppResult {
    let Some(controller) = crate::features::single_instance::context::current() else {
        return wait(launch.shot_at, capture, prof).await;
    };
    capture.prepare().await?;
    prof.mark("capture prepared");
    loop {
        // 1. 【截图延迟】【计划确认】开始之前再次读取最新期限，及时请求和定时到达使用同一状态
        if let Some(plan) = controller.begin() {
            apply(launch, plan);
            clear_terminal();
            prof.idle();
            return Ok(());
        }
        let plan = controller.pending().ok_or("capture instance is closing")?;
        let Some(at) = plan.deadline else {
            continue;
        };
        tokio::select! {
            _ = countdown(at) => {}
            _ = controller.changed() => {}
        }
    }
}

/// 【截图延迟】【直接开始】按 launch 保留未延迟截图的初始参数，返回单实例占用错误，没有控制范围时不改变参数
pub(super) fn begin_launch(launch: &mut super::Launch) -> crate::AppResult {
    if let Some(controller) = crate::features::single_instance::context::current() {
        let plan = controller
            .begin()
            .ok_or("capture instance is not ready to start")?;
        apply(launch, plan);
    }
    Ok(())
}

/// 【截图延迟】【参数应用】按 launch 与 plan 同步实际全屏、多输出和期限，无返回值，其他初始配置继续由原入口负责
fn apply(launch: &mut super::Launch, plan: crate::features::single_instance::request::Plan) {
    launch.fullscreen = plan.fullscreen;
    launch.one_monitor = !plan.all_outputs;
    launch.all_outputs = plan.all_outputs;
    launch.shot_at = plan.deadline;
}

/// 【截图延迟】【独立捕获】按 shot_at、capture 与 prof 准备并等待一次截图，返回准备错误，无延迟时立即继续
pub(super) async fn wait(
    shot_at: Option<Instant>,
    capture: &dyn CaptureMethod,
    prof: &mut Profiler,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(at) = shot_at else {
        return Ok(());
    };
    capture.prepare().await?;
    prof.mark("capture prepared");
    countdown(at).await;
    prof.idle();
    Ok(())
}

/// 【截图延迟】【终端倒计时】按 at 等待目标时间，终端显示剩余秒数，非终端只使用一次计时等待，无返回值
async fn countdown(at: Instant) {
    if !std::io::stderr().is_terminal() {
        return tokio::time::sleep_until(at.into()).await;
    }
    loop {
        let now = Instant::now();
        if now >= at {
            break;
        }
        let left = at - now;
        eprint!(
            "\rscreenshot will be taken in {:.2}s\x1b[K",
            left.as_secs_f64()
        );
        let to_tick = left.as_nanos() % TICK.as_nanos();
        tokio::time::sleep_until((now + Duration::from_nanos(to_tick as u64)).into()).await;
    }
    clear_terminal();
}

/// 【截图延迟】【终端清理】无参数、无返回值，只在实际终端清理完成或替换后的倒计时文字
fn clear_terminal() {
    if std::io::stderr().is_terminal() {
        eprint!("\r\x1b[K");
    }
}
