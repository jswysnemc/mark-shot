//! 【截图覆盖层】【背景模式】在独立冻结背景与完整合成帧之间选择，角色切换不修改已有背景内容

use super::WaylandOverlay;

impl WaylandOverlay {
    /// 【截图覆盖层】【背景模式】按enabled切换背景来源，返回实际启用状态；已有独立背景时保留原有资源契约
    pub(super) fn select_frame_mode(&mut self, enabled: bool) -> bool {
        if enabled
            && self
                .runtime
                .state
                .surfaces
                .values()
                .any(|surface| surface.background.is_some())
        {
            return false;
        }
        self.composited_background = enabled;
        enabled
    }
}
