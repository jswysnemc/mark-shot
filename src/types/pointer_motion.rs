//! 【截图输入】【移动范围】保存同输出水平或垂直共线轨迹端点，非共线位置分别记录，用于精确点击位移判断

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerMotion {
    pub monitor_idx: usize,
    pub min: (f64, f64),
    pub max: (f64, f64),
}

impl PointerMotion {
    /// 【截图输入】【初始范围】按 monitor_idx 与 point 返回单点范围，不分配轨迹列表
    pub fn new(monitor_idx: usize, point: (f64, f64)) -> Self {
        Self {
            monitor_idx,
            min: point,
            max: point,
        }
    }

    /// 【截图输入】【范围合并】将同一输出的 other 纳入当前范围，无返回值，调用方负责核对输出身份
    pub fn include(&mut self, other: Self) {
        self.min = (self.min.0.min(other.min.0), self.min.1.min(other.min.1));
        self.max = (self.max.0.max(other.max.0), self.max.1.max(other.max.1));
    }
}
