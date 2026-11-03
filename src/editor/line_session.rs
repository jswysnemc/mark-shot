//! 【线条】【编辑会话】保存当前定点及双击状态，不进入偏好或撤销快照

use std::time::{Duration, Instant};

use crate::features::line::{Point, polyline::distance};

#[derive(Default)]
pub struct LineSession {
    pub selected_point: Option<(u64, usize)>,
    last_click: Option<(u64, Point, Instant)>,
}

impl LineSession {
    /// 【线条】【双击判断】记录 id、point、now 的按下事件；按已有输入配置返回同一标注是否再次点击
    pub fn register_click(&mut self, id: u64, point: Point, now: Instant) -> bool {
        let double = self
            .last_click
            .is_some_and(|(before_id, before_point, before_time)| {
                let input = &crate::config::get().input;
                before_id == id
                    && now.saturating_duration_since(before_time)
                        <= Duration::from_millis(input.double_click_ms)
                    && distance(before_point, point) <= input.double_click_distance
            });
        self.last_click = if double { None } else { Some((id, point, now)) };
        double
    }

    /// 【线条】【手势结束】清除双击候选，保留选中定点供删除；无返回值
    pub fn clear_click(&mut self) {
        self.last_click = None;
    }

    /// 【线条】【会话清除】工具、选择或历史改变时清除全部临时定点状态；无返回值
    pub fn clear(&mut self) {
        self.selected_point = None;
        self.last_click = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【线条会话测试】【双击时序】只识别同一标注、相近位置和时间，不把第三次单击重复视为第二次双击
    #[test]
    fn double_click_requires_the_same_target_location_and_interval() {
        let mut session = LineSession::default();
        let now = Instant::now();
        assert!(!session.register_click(1, (20.0, 30.0), now));
        assert!(session.register_click(1, (21.0, 30.0), now + Duration::from_millis(100)));
        assert!(!session.register_click(1, (21.0, 30.0), now + Duration::from_millis(150)));
        assert!(!session.register_click(2, (21.0, 30.0), now + Duration::from_millis(200)));
        assert!(!session.register_click(2, (50.0, 30.0), now + Duration::from_millis(250)));
        assert!(!session.register_click(2, (50.0, 30.0), now + Duration::from_millis(900)));
        session.clear();
        assert!(!session.register_click(2, (50.0, 30.0), now + Duration::from_millis(950)));
    }
}
