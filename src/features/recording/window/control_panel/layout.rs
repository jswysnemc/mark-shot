//! 【录制控制面板】【紧凑布局】状态、时长和三个按钮共享固定逻辑尺寸与命中规则

use tiny_skia::Rect;

pub(super) const SIZE: (u32, u32) = (360, 90);
pub(super) const CONTROLS: [Control; 3] = [Control::Toggle, Control::Stop, Control::Hide];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Control {
    Toggle,
    Stop,
    Hide,
}

impl Control {
    /// 【录制控制面板】【按钮范围】无参数，返回当前按钮范围，供绘制与指针命中共用
    pub(super) fn bounds(self) -> Rect {
        let x = match self {
            Self::Toggle => 138.0,
            Self::Stop => 208.0,
            Self::Hide => 278.0,
        };
        Rect::from_xywh(x, 12.0, 64.0, 28.0).unwrap()
    }
}

/// 【录制控制面板】【按钮命中】按 position 返回所在按钮，空白区域用于拖动窗口
pub(super) fn hit(position: (f64, f64)) -> Option<Control> {
    CONTROLS.into_iter().find(|control| {
        let rect = control.bounds();
        position.0 >= f64::from(rect.left())
            && position.0 < f64::from(rect.right())
            && position.1 >= f64::from(rect.top())
            && position.1 < f64::from(rect.bottom())
    })
}
