//! 【输入法】【目标身份】会话、字段焦点和原生焦点使用独立身份，缓存编号不能作为输入目标

use std::sync::atomic::{AtomicU64, Ordering};

use crate::types::text_input::{ChangeCause, FieldLayout, Focus, Snapshot, Update};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// 【输入法】【身份分配】返回进程内不复用的会话或焦点编号，无参数；不依赖标注缓存编号
pub fn identity() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Default)]
pub struct Target {
    pub focus: Option<Focus>,
    pub session_id: Option<u64>,
    pub id: Option<u64>,
    pub snapshot: Option<Snapshot>,
    pub sync_allowed: bool,
    pub cause: ChangeCause,
    pub field_layout: Option<FieldLayout>,
}

impl Target {
    /// 【输入法】【焦点进入】保存 focus 并使旧目标失效，无返回值；进入同一表面也不能复用旧焦点身份
    pub fn enter(&mut self, focus: Focus) {
        self.focus = Some(focus);
        self.session_id = None;
        self.id = None;
        self.snapshot = None;
        self.field_layout = None;
    }

    /// 【输入法】【焦点离开】仅在 id 对应当前原生焦点时清理状态，返回是否匹配；延迟离开不能清理新焦点
    pub fn leave(&mut self, id: u64) -> bool {
        if self.focus.is_none_or(|focus| focus.id != id) {
            return false;
        }
        self.focus = None;
        self.session_id = None;
        self.id = None;
        self.snapshot = None;
        self.field_layout = None;
        true
    }

    /// 【输入法】【字段切换】按 session_id 更新正文输入身份，返回目标是否改变；空值表示非正文控件获得输入
    pub fn reconcile(&mut self, session_id: Option<u64>) -> bool {
        let session_id = session_id.filter(|_| self.focus.is_some());
        if self.session_id == session_id {
            return false;
        }
        self.session_id = session_id;
        self.id = session_id.map(|_| identity());
        self.snapshot = None;
        self.field_layout = None;
        self.sync_allowed = true;
        self.cause = ChangeCause::Other;
        true
    }

    /// 【输入法】【批次接收】核验 update 的目标身份并更新序号同步许可，返回是否属于当前字段；过期消息不改变任何状态
    pub fn accept(&mut self, update: &Update) -> bool {
        if self.id != Some(update.target_id) {
            return false;
        }
        self.sync_allowed = update.sync_allowed;
        true
    }
}
