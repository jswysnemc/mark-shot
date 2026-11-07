//! 【滚动截图】【表面角色】只映射选区所属输出，底层窗口继续接收键盘与滚轮

use super::WaylandOverlay;

impl WaylandOverlay {
    /// 【滚动截图】【角色切换】以 monitor_idx 保留原输出身份并重建按需焦点浮层，返回协议结果
    pub(super) fn start_scroll_surface(&mut self, monitor_idx: usize) -> crate::AppResult {
        if self.targets.get(monitor_idx).is_none() {
            return Err("the scroll capture output is no longer available".into());
        }
        // 1. 【滚动截图】【输入释放】先销毁独占焦点的原表面，再建立按需焦点表面
        self.suspend_surfaces()?;
        self.scrolling = Some(monitor_idx);
        self.resume_surfaces()?;
        log::info!("【滚动截图】【输入角色】已释放独占键盘，目标表面={monitor_idx}");
        Ok(())
    }
}
