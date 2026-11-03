//! 【字号历史】【连续滚轮】防抖合并同一文字目标的修改，保留独立输入余量，不增加空闲轮询

use super::{EditorState, history_snapshot::HistorySnapshot};
use crate::features::text::size_input::SizeInput;
use std::time::{Duration, Instant};

pub const DEBOUNCE: Duration = Duration::from_millis(350);

#[derive(Default)]
pub struct TextSizeHistory {
    pub input: SizeInput,
    pub context: Option<Option<u64>>,
    pub pending: Option<HistorySnapshot>,
    pub deadline: Option<Instant>,
}

impl EditorState {
    /// 【字号历史】【挂起提交】提交当前画布滚轮快照，不清除输入余量；无参数、无返回值
    pub fn commit_text_size_wheel(&mut self) {
        self.text_size_history.deadline = None;
        let snapshot = self.text_size_history.pending.take();
        self.commit_snapshot(snapshot);
    }

    /// 【字号历史】【输入准备】按 context 目标和 now 时间提交上一轮历史，更换目标才清除余量；无返回值
    pub fn prepare_text_size_wheel(&mut self, context: Option<u64>, now: Instant) {
        if self.text_size_history.context != Some(context)
            || self
                .text_size_history
                .deadline
                .is_some_and(|deadline| now >= deadline)
        {
            self.commit_text_size_wheel();
        }
        if self.text_size_history.context != Some(context) {
            self.text_size_history.input.reset();
            self.text_size_history.context = Some(context);
        }
    }

    /// 【字号历史】【防抖记录】用 snapshot 保存 state 第一次实际变更前的标注，now 延长提交期限；无返回值
    pub fn queue_text_size_wheel(&mut self, snapshot: HistorySnapshot, now: Instant) {
        self.text_size_history.pending.get_or_insert(snapshot);
        self.text_size_history.deadline = Some(now + DEBOUNCE);
        self.redo_stack.clear();
    }

    /// 【字号历史】【到期提交】在 now 达到期限时提交历史；无返回值，不清除小刻度余量
    pub fn tick_text_size_wheel(&mut self, now: Instant) {
        if self
            .text_size_history
            .deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.commit_text_size_wheel();
        }
    }
}
