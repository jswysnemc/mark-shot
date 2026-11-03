//! 【文字光标】【相位计时】只保存可见相位与下一截止时间，未聚焦时不保留周期唤醒

use std::time::{Duration, Instant};

pub const FLASH_PERIOD: Duration = Duration::from_millis(1000);

pub struct Caret {
    half_period: Duration,
    active: bool,
    visible: bool,
    deadline: Option<Instant>,
}

impl Default for Caret {
    /// 【文字光标】【默认周期】返回当前原版一秒周期的未聚焦状态，无参数
    fn default() -> Self {
        Self::new(FLASH_PERIOD)
    }
}

impl Caret {
    /// 【文字光标】【周期构造】按 period 返回未聚焦光标；零周期表示聚焦后常亮，不产生计时器
    pub fn new(period: Duration) -> Self {
        Self {
            half_period: period / 2,
            active: false,
            visible: false,
            deadline: None,
        }
    }

    /// 【文字光标】【焦点同步】用 active 与 now 更新焦点；获得焦点重启周期，失去焦点隐藏，返回可见性是否变化
    pub fn focus(&mut self, active: bool, now: Instant) -> bool {
        if self.active == active {
            return false;
        }
        self.active = active;
        let changed = self.visible != active;
        self.visible = active;
        self.deadline = (active && !self.half_period.is_zero()).then(|| now + self.half_period);
        changed
    }

    /// 【文字光标】【编辑活动】输入立即显示当前聚焦光标，但不推迟原版周期，返回可见性是否变化
    pub fn activity(&mut self) -> bool {
        let changed = self.active && !self.visible;
        self.visible = self.active;
        changed
    }

    /// 【文字光标】【鼠标活动】正文按下或释放时用 now 重新建立周期，返回可见性是否变化；字段按钮不调用此入口
    pub fn pointer_activity(&mut self, now: Instant) -> bool {
        let changed = self.activity();
        if self.active && !self.half_period.is_zero() {
            self.deadline = Some(now + self.half_period);
        }
        changed
    }

    /// 【文字光标】【截止处理】now 到期后切换一次；延迟处理不补发密集帧，返回可见性是否变化
    pub fn tick(&mut self, now: Instant) -> bool {
        if self.deadline.is_none_or(|deadline| now < deadline) {
            return false;
        }
        self.visible = !self.visible;
        self.deadline = Some(now + self.half_period);
        true
    }

    /// 【文字光标】【可见读取】返回当前相位，不在绘制时修改周期，无参数
    pub fn visible(&self) -> bool {
        self.visible
    }

    /// 【文字光标】【唤醒读取】返回下一截止时间；失去焦点或零周期时无定时唤醒，无参数
    pub fn deadline(&self) -> Option<Instant> {
        self.deadline
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【文字光标测试】【半周期】一秒周期每五百毫秒切换，到期前不重绘，无参数、无返回值
    #[test]
    fn focus_and_half_period_match_the_qt_trace() {
        let now = Instant::now();
        let mut caret = Caret::default();
        assert!(caret.focus(true, now));
        assert!(caret.visible());
        assert_eq!(caret.deadline(), Some(now + Duration::from_millis(500)));
        assert!(!caret.tick(now + Duration::from_millis(499)));
        assert!(caret.tick(now + Duration::from_millis(500)));
        assert!(!caret.visible());
        assert!(caret.tick(now + Duration::from_millis(1000)));
        assert!(caret.visible());
    }

    /// 【文字光标测试】【输入周期】输入恢复可见但不重置截止；下一次到期仍切换，无参数、无返回值
    #[test]
    fn activity_restores_visibility_without_postponing_the_timer() {
        let now = Instant::now();
        let mut caret = Caret::default();
        caret.focus(true, now);
        caret.tick(now + Duration::from_millis(500));
        assert!(caret.activity());
        assert_eq!(caret.deadline(), Some(now + Duration::from_millis(1000)));
        assert!(caret.tick(now + Duration::from_millis(1000)));
        assert!(!caret.visible());
    }

    /// 【文字光标测试】【鼠标周期】正文按下与释放重建半周期，不能沿用键盘输入的不推迟策略，无参数、无返回值
    #[test]
    fn pointer_input_restarts_the_caret_timer() {
        let now = Instant::now();
        let mut caret = Caret::default();
        caret.focus(true, now);
        caret.tick(now + Duration::from_millis(500));
        assert!(caret.pointer_activity(now + Duration::from_millis(700)));
        assert_eq!(caret.deadline(), Some(now + Duration::from_millis(1200)));
        assert!(!caret.tick(now + Duration::from_millis(1000)));
    }

    /// 【文字光标测试】【焦点退出】失去焦点清除截止与可见相位，隐藏期间输入不启动计时，返回后新建周期，无参数、无返回值
    #[test]
    fn inactive_caret_has_no_deadline_or_activity() {
        let now = Instant::now();
        let mut caret = Caret::default();
        caret.focus(true, now);
        assert!(caret.focus(false, now));
        assert!(!caret.activity());
        assert!(!caret.visible());
        assert_eq!(caret.deadline(), None);
        assert!(!caret.tick(now + Duration::from_secs(3)));
        caret.focus(true, now + Duration::from_secs(3));
        assert_eq!(caret.deadline(), Some(now + Duration::from_millis(3500)));
    }

    /// 【文字光标测试】【延迟合并】处理过期计时只切换一次，下一截止严格晚于当前时间，无参数、无返回值
    #[test]
    fn overdue_timers_do_not_create_a_redraw_burst() {
        let now = Instant::now();
        let mut caret = Caret::default();
        caret.focus(true, now);
        assert!(caret.tick(now + Duration::from_secs(8)));
        assert!(!caret.tick(now + Duration::from_secs(8)));
        assert_eq!(caret.deadline(), Some(now + Duration::from_millis(8500)));
    }

    /// 【文字光标测试】【零周期】关闭闪烁仍显示聚焦光标，不保留周期唤醒，无参数、无返回值
    #[test]
    fn disabled_blinking_is_steady_without_polling() {
        let now = Instant::now();
        let mut caret = Caret::new(Duration::ZERO);
        caret.focus(true, now);
        assert!(caret.visible());
        assert_eq!(caret.deadline(), None);
        assert!(!caret.tick(now + Duration::from_secs(5)));
    }

    /// 【文字光标测试】【原版采样】从真实 Qt fixture 验证周期、选区闪烁与焦点暂停，不用手写采样代替原版结果，无参数、无返回值
    #[test]
    fn frozen_qt_samples_prove_the_same_period_and_focus_contract() {
        let samples: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/qt-text-caret.json"))
                .unwrap();
        assert_eq!(
            FLASH_PERIOD.as_millis(),
            samples["flash_ms"].as_u64().unwrap() as u128
        );
        for field in ["typing", "selected", "focus_return", "menu_return"] {
            let values = samples[field].as_array().unwrap();
            assert!(
                values
                    .iter()
                    .any(|sample| sample["caret_pixels"].as_u64().unwrap() > 0),
                "{field}"
            );
            assert!(
                values.iter().any(|sample| sample["caret_pixels"] == 0),
                "{field}"
            );
        }
        for field in ["field_focus", "focus_out", "menu_focus"] {
            assert!(
                samples[field]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|sample| sample["caret_pixels"] == 0),
                "{field}"
            );
        }
        let typing = samples["typing"].as_array().unwrap();
        let first = typing
            .iter()
            .find(|sample| sample["caret_pixels"] == 0)
            .unwrap()["ms"]
            .as_u64()
            .unwrap();
        assert!((400..=650).contains(&first));
        let visible = typing
            .iter()
            .find(|sample| sample["caret_pixels"].as_u64().unwrap() > 0)
            .unwrap();
        assert_eq!(
            visible["center_rgba"],
            serde_json::json!([255, 77, 77, 255])
        );
    }
}
