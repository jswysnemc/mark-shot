//! 【录制窗口】【流程编排】准备窗口、倒计时小组件与后台会话顺序衔接，界面关闭后录制继续

mod background;
mod countdown;
mod fonts;
mod host;
mod preparation;
mod setup;
mod summary;

use super::Target;
use crate::config::recording::Recording;
use host::{Host, Shared};
use std::{cell::RefCell, rc::Rc, time::Duration};

/// 小组件销毁请求发出后等待合成器刷新一帧的时间，保证采集的第一帧不含小组件
const SETTLE: Duration = Duration::from_millis(60);

/// 【录制窗口】【目标摘要】按 host 和 config 返回倒计时小组件显示的目标与格式说明
fn describe(host: &Shared, config: &Recording) -> String {
    let cn = crate::ui::language::Language::current() == crate::ui::language::Language::Chinese;
    // 格式与帧率在前，目标说明较长时由小组件截断末尾
    format!(
        "{} · {}",
        summary::format(config),
        summary::target(host.borrow().target(), cn)
    )
}

/// 【录制窗口】【启动】按 target 进入录制流程，有本次参数时跳过准备窗口，返回配置、独占锁或平台错误
///
/// 独立入口取消或失败时回到准备窗口；工具入口取消后退出，失败返回错误；保存成功后退出
pub(super) fn run(target: Target) -> crate::AppResult {
    let mut initial = target.config.clone();
    let from_tool = initial.is_some();
    if let Some(config) = &initial {
        config.validate()?;
    }
    let host: Shared = Rc::new(RefCell::new(Host::new(target)?));
    let fonts = fonts::shared();
    let mut failure = None;
    loop {
        // 1. 【录制窗口】【准备阶段】用户确认选项；直接关闭或转去框选区域时结束本进程
        let config = if let Some(config) = initial.take() {
            config
        } else {
            setup::run(host.clone(), fonts.clone(), failure.take())?;
            let Some(config) = host.borrow_mut().take_request() else {
                return Ok(());
            };
            config
        };
        host.borrow_mut().prepare(&config);
        // 2. 【录制窗口】【倒计时阶段】只在设置了倒计时时显示小组件；取消回到准备窗口，小组件故障不阻止录制
        if config.countdown > 0 {
            let duration = Duration::from_secs(u64::from(config.countdown));
            match countdown::run(
                host.clone(),
                fonts.clone(),
                duration,
                describe(&host, &config),
            ) {
                Ok(true) => std::thread::sleep(SETTLE),
                Ok(false) => {
                    host.borrow_mut().reset();
                    if from_tool {
                        return Ok(());
                    }
                    continue;
                }
                Err(error) => log::error!("【录制窗口】【倒计时小组件】{error}"),
            }
        }
        // 3. 【录制窗口】【后台阶段】没有任何界面，暂停、继续和停止来自截图界面、托盘或命令行
        if let Err(error) = host.borrow_mut().start(config) {
            if from_tool {
                return Err(error.into());
            }
            failure = Some(error);
            continue;
        }
        let status = background::wait(&host);
        match status.phase.as_str() {
            "finished" => {
                background::finished(&status);
                return Ok(());
            }
            "cancelled" if from_tool => return Ok(()),
            "cancelled" => {}
            _ => {
                let error = if status.detail.is_empty() {
                    status.phase.clone()
                } else {
                    status.detail.clone()
                };
                if from_tool {
                    return Err(error.into());
                }
                failure = Some(error);
            }
        }
        // 4. 【录制窗口】【返回准备】取消或失败后释放已结束的会话，保留目标供再次开始
        host.borrow_mut().reset();
    }
}
