//! 【编辑历史】【统一快照】把持久标注和序号计数作为同一次撤销状态保存

use crate::features::number::NumberSequence;
use crate::types::Annotation;

#[derive(Clone, Debug, PartialEq)]
pub struct HistorySnapshot {
    pub annotations: Vec<Annotation>,
    pub number_sequence: NumberSequence,
    pub selected_id: Option<u64>,
    pub next_id: u64,
}
