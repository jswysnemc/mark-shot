//! 【截图覆盖层】【可见性】独立对话框期间释放表面和输入，恢复时保留原输出身份

use crate::backend::ScreenOverlay;

use super::WaylandOverlay;

impl WaylandOverlay {
    /// 【截图覆盖层】【表面暂停】移除当前表面并确认服务端已处理；不释放调用方的冻结图像，返回协议结果
    pub(super) fn suspend_surfaces(&mut self) -> crate::AppResult {
        if self.suspended {
            return Ok(());
        }
        let runtime = &mut self.runtime;
        runtime.state.popup = None;
        // 1. 【截图覆盖层】【资源移除】先释放子表面和视区，再由角色所有者销毁角色及主表面
        for (_, mut surface) in std::mem::take(&mut runtime.state.surfaces) {
            surface.surface.attach(None, 0, 0);
            surface.surface.commit();
            surface.background = None;
            surface.scroll_layer = None;
            if let Some(viewport) = surface.viewport.take() {
                viewport.destroy();
            }
        }
        runtime.state.pointer_surface_idx = None;
        runtime.event_queue.flush()?;
        runtime.event_queue.roundtrip(&mut runtime.state)?;
        // 2. 【截图覆盖层】【共享缓冲回收】表面关闭后重建空池，避免旧整屏页留到下次窗口创建
        runtime.state.pool =
            smithay_client_toolkit::shm::slot::SlotPool::new(256 * 1024, &runtime.state.shm)?;
        self.faulted = 0;
        // 2. 【截图覆盖层】【旧事件清理】旧表面的焦点和帧事件不能影响随后打开的文件对话框
        runtime.state.events.clear();
        runtime.state.popup_grabs.clear();
        self.suspended = true;
        Ok(())
    }

    /// 【截图覆盖层】【表面恢复】重新读取输出并按身份创建原表面；输出已经消失时报告错误
    pub(super) fn resume_surfaces(&mut self) -> crate::AppResult {
        if !self.suspended {
            return Ok(());
        }
        self.runtime
            .event_queue
            .roundtrip(&mut self.runtime.state)?;
        let targets = self
            .targets
            .iter()
            .map(|target| {
                self.runtime
                    .state
                    .outputs
                    .iter()
                    .position(|output| output.wl_output == *target)
                    .ok_or("the original capture output is no longer available")
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.present(&targets)?;
        self.suspended = false;
        Ok(())
    }
}
