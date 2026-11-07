//! 【标注样式】【窗口限制】绘制限制使用窗口逻辑像素，不写回持久笔宽或属性默认值

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowUnits {
    canvas_per_window: f32,
}

impl Default for WindowUnits {
    /// 【标注样式】【默认窗口】无参数，返回没有持续导航倍率的窗口单位
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl WindowUnits {
    /// 【标注样式】【窗口构造】按canvas_per_window返回窗口单位，无效比例使用一比一，不读取设备像素比例
    pub fn new(canvas_per_window: f32) -> Self {
        let canvas_per_window = if canvas_per_window.is_finite() && canvas_per_window > 0.0 {
            canvas_per_window
        } else {
            1.0
        };
        Self { canvas_per_window }
    }

    /// 【标注样式】【窗口到画布】按window_value返回绘制时使用的稳定画布尺寸
    pub fn canvas(self, window_value: f32) -> f32 {
        window_value * self.canvas_per_window
    }

    /// 【标注样式】【画布到窗口】按canvas_value返回当前窗口逻辑尺寸，不加入输出设备比例
    pub fn window(self, canvas_value: f32) -> f32 {
        canvas_value / self.canvas_per_window
    }

    /// 【标注样式】【窗口笔宽】按canvas_width、minimum和maximum返回有效绘制笔宽，限制值使用窗口单位，不修改原图数值
    pub fn stroke(self, canvas_width: f32, minimum: f32, maximum: Option<f32>) -> f32 {
        let minimum = self.canvas(minimum);
        let width = canvas_width.max(minimum);
        maximum.map_or(width, |maximum| {
            width.min(self.canvas(maximum).max(minimum))
        })
    }

    /// 【标注样式】【窗口字体】按canvas_points返回画布中的字体像素尺寸，先取整窗口点字号，再使用原版九十六DPI取整像素
    pub fn font_pixels(self, canvas_points: f32) -> f32 {
        let window_points = self.window(canvas_points).round().max(1.0);
        self.canvas((window_points * 96.0 / 72.0).round())
    }
}
