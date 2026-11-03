//! 【截图历史】【水平分隔】管理固定左栏、最小预览及按下位置，不改变原图比例

#[derive(Clone, Copy)]
pub(super) struct Drag {
    origin: f64,
    width: f32,
}

/// 【截图历史】【分隔夹紧】把 width 限制在 body_width 内，返回满足原版最小列表与预览宽度的值
pub(super) fn clamp(width: f32, body_width: f32) -> f32 {
    width.clamp(184.0, (body_width - 179.0).max(184.0))
}

impl Drag {
    /// 【截图历史】【分隔开始】记录 position 的横坐标与当前 width，返回本次拖动状态
    pub fn new(position: (f64, f64), width: f32) -> Self {
        Self {
            origin: position.0,
            width,
        }
    }

    /// 【截图历史】【分隔移动】按 position 及 body_width 返回夹紧后的左栏宽度，不累积四舍五入误差
    pub fn width(self, position: (f64, f64), body_width: f32) -> f32 {
        clamp(self.width + (position.0 - self.origin) as f32, body_width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【截图历史测试】【分隔边界】极端拖动保留两侧最小尺寸，反向移动从原按下点计算，无参数、无返回值
    #[test]
    fn split_drag_retains_both_panes_and_does_not_drift() {
        let drag = Drag::new((221.0, 100.0), 210.0);
        assert_eq!(drag.width((-100.0, 100.0), 416.0), 184.0);
        assert_eq!(drag.width((900.0, 100.0), 416.0), 237.0);
        assert_eq!(drag.width((241.0, 100.0), 416.0), 230.0);
        assert_eq!(drag.width((221.0, 100.0), 416.0), 210.0);
    }
}
