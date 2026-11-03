//! 【字号输入】【余量规则】画布、滑条与数字框按真实 Qt 入口分别累积，不共用反向清零规则

#[derive(Debug, Default)]
pub struct SizeInput {
    remainder: f64,
}

impl SizeInput {
    /// 【字号输入】【余量读取】返回尚未转换为整点的输入余量
    pub fn remainder(&self) -> f64 {
        self.remainder
    }

    /// 【字号输入】【上下文重置】清除旧目标余量，无参数、无返回值
    pub fn reset(&mut self) {
        self.remainder = 0.0;
    }

    /// 【字号输入】【画布增量】按 steps 刻度计算整点增量；每刻度一点五点，方向和时间间隔不清除余量
    pub fn canvas(&mut self, steps: f64) -> i32 {
        self.accumulate(steps * 1.5, false)
    }

    /// 【字号输入】【滑条增量】按 angle 角度计算数值增量；普通输入每刻度三格，page 表示修饰键翻页并使用 page_step，不累积翻页余量
    pub fn slider(&mut self, angle: f64, page: bool, page_step: i32) -> i32 {
        if page {
            self.reset();
            return (angle / 120.0 * page_step as f64)
                .clamp(-page_step as f64, page_step as f64)
                .trunc() as i32;
        }
        self.accumulate(angle / 120.0 * 3.0, true)
            .clamp(-page_step, page_step)
    }

    /// 【字号输入】【数字框增量】按 angle 角度累积完整刻度，control 为真时每刻度十点，否则每刻度一点
    pub fn spin(&mut self, angle: f64, control: bool) -> i32 {
        self.accumulate(angle / 120.0, false)
            .saturating_mul(if control { 10 } else { 1 })
    }

    /// 【字号输入】【整点提取】累积 delta 并朝零截断，reverse_reset 决定反向清除余量；返回整数增量，非有限值不改变状态
    fn accumulate(&mut self, delta: f64, reverse_reset: bool) -> i32 {
        if !delta.is_finite() || delta == 0.0 {
            return 0;
        }
        if reverse_reset && self.remainder * delta < 0.0 {
            self.reset();
        }
        let raw = self.remainder + delta;
        let whole = raw.trunc();
        self.remainder = raw - whole;
        whole as i32
    }
}
