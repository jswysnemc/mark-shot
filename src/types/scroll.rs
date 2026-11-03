//! 【滚轮输入】【轴数据】保留像素、刻度与来源，纵轴正值表示向上，不再次应用自然滚动方向

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollSource {
    Wheel,
    Finger,
    Continuous,
    WheelTilt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Identical,
    Inverted,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ScrollAxis {
    pub pixels: f64,
    pub discrete: i32,
    pub value120: i32,
    pub angle_delta: Option<i32>,
    pub direction: Option<ScrollDirection>,
    pub stop: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ScrollEvent {
    pub horizontal: ScrollAxis,
    pub vertical: ScrollAxis,
    pub source: Option<ScrollSource>,
}

impl ScrollEvent {
    /// 【滚轮输入】【刻度构造】按 value120 返回纵向鼠标滚轮事件；每一百二十单位代表一刻度，不伪造像素值
    pub fn wheel(value120: i32) -> Self {
        Self {
            vertical: ScrollAxis {
                value120,
                ..Default::default()
            },
            source: Some(ScrollSource::Wheel),
            ..Default::default()
        }
    }

    /// 【滚轮输入】【像素构造】按 x、y 返回明确没有角度的事件，用于复现纯像素控件输入，不代替 Wayland 平台转换
    pub fn pixels(x: f64, y: f64) -> Self {
        Self {
            horizontal: ScrollAxis {
                pixels: x,
                ..Default::default()
            },
            vertical: ScrollAxis {
                pixels: y,
                angle_delta: Some(0),
                ..Default::default()
            },
            source: Some(ScrollSource::Finger),
        }
    }

    /// 【滚轮输入】【角度读取】返回纵轴角度单位；高分辨率优先于旧刻度，无刻度时按 Qt Wayland 规则从所有来源的像素合成角度，显式纯像素事件除外
    pub fn angle_y(self) -> f64 {
        if self.vertical.value120 != 0 {
            self.vertical.value120 as f64
        } else if self.vertical.discrete != 0 {
            self.vertical.discrete as f64 * 120.0
        } else {
            self.vertical.angle_delta.map_or_else(
                || (finite(self.vertical.pixels) * 12.0).round(),
                |angle| angle as f64,
            )
        }
    }

    /// 【滚轮输入】【画布刻度】返回字号画布步进，角度优先，显式纯像素按八十单位一刻度换算；平台角度取整为零的微小位移不累积
    pub fn canvas_steps(self) -> f64 {
        let angle = self.angle_y();
        if angle != 0.0 {
            angle / 120.0
        } else if self.vertical.angle_delta == Some(0) {
            finite(self.vertical.pixels) / 80.0
        } else {
            0.0
        }
    }

    /// 【滚轮输入】【兼容像素】返回旧工具所需纵轴像素；只有刻度时按每刻度十像素转换，不丢失高分辨率小数
    pub fn legacy_y(self) -> f32 {
        if self.vertical.pixels != 0.0 {
            finite(self.vertical.pixels) as f32
        } else {
            (self.angle_y() / 12.0) as f32
        }
    }

    /// 【滚轮输入】【事件判断】返回是否包含轴运动、方向信息或停止信号，停止事件仍进入输入分派
    pub fn is_empty(self) -> bool {
        [self.horizontal, self.vertical].iter().all(|axis| {
            axis.pixels == 0.0
                && axis.value120 == 0
                && axis.discrete == 0
                && axis.direction.is_none()
                && !axis.stop
        })
    }
}

/// 【滚轮输入】【有限校验】返回 value 的有限值；非有限输入返回零，避免余量永久失效
fn finite(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}
