//! 【原生窗口】【浮层几何】结果浮层保留逻辑尺寸和位置，协议边距使用输出本地坐标

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Geometry {
    pub position: (i32, i32),
    pub size: (u32, u32),
    pub output_origin: (i32, i32),
    pub initialized: bool,
}

impl Geometry {
    /// 【原生窗口】【初始浮层】按 size 创建等待输出定位的几何，首次由合成器居中，返回状态
    pub fn new(size: (u32, u32)) -> Self {
        Self {
            position: (0, 0),
            size,
            output_origin: (0, 0),
            initialized: false,
        }
    }

    /// 【原生窗口】【输出居中】按 origin、bounds 在输出内居中当前尺寸，返回初始化完成的几何
    pub fn centered(mut self, origin: (i32, i32), bounds: (i32, i32)) -> Self {
        self.output_origin = origin;
        self.size = (
            self.size.0.min(bounds.0.max(1) as u32),
            self.size.1.min(bounds.1.max(1) as u32),
        );
        self.position = (
            origin.0 + (bounds.0 - 1) / 2 - (self.size.0 as i32 - 1) / 2,
            origin.1 + (bounds.1 - 1) / 2 - (self.size.1 as i32 - 1) / 2,
        );
        self.initialized = true;
        self
    }

    /// 【原生窗口】【本地边距】返回协议需要的左、上边距，保留负边距以支持离开输出边界，无参数
    pub fn margin(self) -> (i32, i32) {
        (
            self.position.0 - self.output_origin.0,
            self.position.1 - self.output_origin.1,
        )
    }
}

#[derive(Clone, Copy)]
pub(super) struct Gesture {
    initial: Geometry,
    pointer: (f64, f64),
    resize: bool,
}

impl Gesture {
    /// 【原生窗口】【尺寸事务】返回本事务是否调整尺寸，移动事务保持独立光标行为，无参数
    pub fn resizing(self) -> bool {
        self.resize
    }

    /// 【原生窗口】【浮层按下】按 geometry、local、resize 保存逻辑全局起点，返回拖动或右下尺寸事务
    pub fn new(geometry: Geometry, local: (f64, f64), resize: bool) -> Self {
        Self {
            initial: geometry,
            pointer: (
                f64::from(geometry.position.0) + local.0,
                f64::from(geometry.position.1) + local.1,
            ),
            resize,
        }
    }

    /// 【原生窗口】【浮层移动】按 current、local 和 minimum 计算新几何，local 使用按下表面的固定抓取坐标，返回位置或尺寸
    pub fn update(self, current: Geometry, local: (f64, f64), minimum: (u32, u32)) -> Geometry {
        let delta = (
            (f64::from(self.initial.position.0) + local.0 - self.pointer.0).round() as i32,
            (f64::from(self.initial.position.1) + local.1 - self.pointer.1).round() as i32,
        );
        let mut geometry = current;
        if self.resize {
            geometry.size = (
                (i64::from(self.initial.size.0) + i64::from(delta.0))
                    .clamp(i64::from(minimum.0), 8192) as u32,
                (i64::from(self.initial.size.1) + i64::from(delta.1))
                    .clamp(i64::from(minimum.1), 8192) as u32,
            );
        } else {
            geometry.position = (
                self.initial.position.0.saturating_add(delta.0),
                self.initial.position.1.saturating_add(delta.1),
            );
        }
        geometry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【浮层几何测试】【输出局部】非零及负输出位置不进入协议边距，尺寸始终受输出约束，无参数、无返回值
    #[test]
    fn centering_and_margins_use_the_target_output_coordinates() {
        let geometry = Geometry::new((640, 520)).centered((-1200, 100), (1000, 700));
        assert_eq!(geometry.position, (-1020, 190));
        assert_eq!(geometry.margin(), (180, 90));
        assert_eq!(
            Geometry::new((640, 520)).centered((0, 0), (400, 360)).size,
            (400, 360)
        );
    }

    /// 【浮层几何测试】【连续指针】隐式抓取坐标使用按下原点，重复通知不能累加位移，尺寸调整保持最小尺寸，无参数、无返回值
    #[test]
    fn moving_and_resizing_use_a_fixed_global_press_origin() {
        let initial = Geometry::new((640, 520)).centered((0, 0), (1000, 700));
        let gesture = Gesture::new(initial, (100.0, 15.0), false);
        let first = gesture.update(initial, (120.0, 35.0), (380, 340));
        assert_eq!(first.position, (200, 110));
        assert_eq!(gesture.update(first, (120.0, 35.0), (380, 340)), first);
        let resize = Gesture::new(initial, (640.0, 520.0), true);
        assert_eq!(
            resize.update(initial, (0.0, 0.0), (380, 340)).size,
            (380, 340)
        );
    }
}
