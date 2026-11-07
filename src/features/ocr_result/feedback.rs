//! 【文字识别】【复制反馈】原文与表格分别管理成功、失败及绝对期限，不由切页或编辑清除

use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub(super) enum Pane {
    Source,
    Table,
    Translation,
}

#[derive(Default)]
pub(super) struct Feedback {
    source: Option<(bool, Instant)>,
    table: Option<(bool, Instant)>,
    translation: Option<(bool, Instant)>,
}

impl Feedback {
    /// 【文字识别】【反馈开始】为 pane 记录 success，now 为确认完成时间；重复复制只重置该分区期限，无返回值
    pub fn show(&mut self, pane: Pane, success: bool, now: Instant) {
        let duration = match pane {
            Pane::Source | Pane::Translation => Duration::from_millis(1800),
            Pane::Table => Duration::from_millis(1600),
        };
        *self.entry(pane) = Some((success, now + duration));
    }

    /// 【文字识别】【分区反馈】按 pane 返回反馈状态的可变位置，不修改其他分区，无其他参数
    fn entry(&mut self, pane: Pane) -> &mut Option<(bool, Instant)> {
        match pane {
            Pane::Source => &mut self.source,
            Pane::Table => &mut self.table,
            Pane::Translation => &mut self.translation,
        }
    }

    /// 【文字识别】【反馈文字】按 pane 返回当前英文标签，不读时钟；到期状态由单次唤醒显式清除
    pub fn label(&self, pane: Pane) -> &'static str {
        let value = match pane {
            Pane::Source => self.source,
            Pane::Table => self.table,
            Pane::Translation => self.translation,
        };
        match value {
            Some((true, _)) => "Copied",
            Some((false, _)) => "Copy failed",
            None => "Copy",
        }
    }

    /// 【文字识别】【最近期限】返回仍有反馈的最早绝对期限，无参数；空值表示不需要计时唤醒
    pub fn deadline(&self) -> Option<Instant> {
        self.source
            .into_iter()
            .chain(self.table)
            .chain(self.translation)
            .map(|(_, deadline)| deadline)
            .min()
    }

    /// 【文字识别】【反馈到期】只清除期限不晚于 now 的分区，返回是否需要重绘；不重置另一分区或安排周期任务
    pub fn expire(&mut self, now: Instant) -> bool {
        let mut changed = false;
        for entry in [&mut self.source, &mut self.table, &mut self.translation] {
            if entry.is_some_and(|(_, deadline)| deadline <= now) {
                *entry = None;
                changed = true;
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【复制反馈测试】【真实 Qt 记录】从完整原版窗口采集读取两个实际计时器间隔，逐项比较原生绝对期限，无参数、无返回值
    #[test]
    fn configured_deadlines_match_the_actual_qt_window_record() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/qt-ocr-titlebar.json"))
                .unwrap();
        let now = Instant::now();
        for (pane, key) in [
            (Pane::Source, "source_interval_ms"),
            (Pane::Table, "table_interval_ms"),
        ] {
            let mut feedback = Feedback::default();
            feedback.show(pane, true, now);
            assert_eq!(
                feedback.deadline(),
                Some(now + Duration::from_millis(fixture["feedback"][key].as_u64().unwrap()))
            );
        }
    }

    /// 【复制反馈测试】【原版期限】原文与表格分别使用 1800、1600 毫秒，表格到期不清除原文，无参数、无返回值
    #[test]
    fn panes_expire_on_their_own_qt_deadlines() {
        let now = Instant::now();
        let mut feedback = Feedback::default();
        feedback.show(Pane::Source, true, now);
        feedback.show(Pane::Table, true, now);
        assert_eq!(feedback.deadline(), Some(now + Duration::from_millis(1600)));
        assert!(!feedback.expire(now + Duration::from_millis(1599)));
        assert!(feedback.expire(now + Duration::from_millis(1600)));
        assert_eq!(feedback.label(Pane::Table), "Copy");
        assert_eq!(feedback.label(Pane::Source), "Copied");
        assert_eq!(feedback.deadline(), Some(now + Duration::from_millis(1800)));
        assert!(feedback.expire(now + Duration::from_millis(1800)));
        assert_eq!(feedback.deadline(), None);
        assert!(!feedback.expire(now + Duration::from_secs(20)));
    }

    /// 【复制反馈测试】【重复操作】重复原文复制只重置原文期限，旧期限不能撤销新失败反馈，无参数、无返回值
    #[test]
    fn renewed_feedback_survives_the_old_deadline() {
        let now = Instant::now();
        let mut feedback = Feedback::default();
        feedback.show(Pane::Source, true, now);
        feedback.show(Pane::Source, false, now + Duration::from_millis(300));
        assert!(!feedback.expire(now + Duration::from_millis(1800)));
        assert_eq!(feedback.label(Pane::Source), "Copy failed");
        assert!(feedback.expire(now + Duration::from_millis(2100)));
        assert_eq!(feedback.label(Pane::Source), "Copy");
    }

    /// 【复制反馈测试】【交叉操作】原文重新复制不能延长或替换表格反馈，无参数、无返回值
    #[test]
    fn copying_source_does_not_reset_the_table_deadline() {
        let now = Instant::now();
        let mut feedback = Feedback::default();
        feedback.show(Pane::Table, false, now);
        feedback.show(Pane::Source, true, now + Duration::from_millis(700));
        assert!(feedback.expire(now + Duration::from_millis(1600)));
        assert_eq!(feedback.label(Pane::Table), "Copy");
        assert_eq!(feedback.label(Pane::Source), "Copied");
        assert_eq!(feedback.deadline(), Some(now + Duration::from_millis(2500)));
    }
}
