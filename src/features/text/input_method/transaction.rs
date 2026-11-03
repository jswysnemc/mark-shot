//! 【输入法】【原子批次】候选、确认和删除在 done 时交付，旧序号仍交付内容但禁止提前重发上下文

use crate::types::text_input::{Batch, Preedit, Update};

#[derive(Default)]
pub struct Pending {
    batch: Batch,
}

impl Pending {
    /// 【输入法】【候选暂存】暂存 text 与 begin、end 字节位置，返回是否合法；非法新候选清除旧候选，不影响同批确认文字
    pub fn preedit(&mut self, text: Option<String>, begin: i32, end: i32) -> bool {
        self.batch.preedit = Preedit::default();
        let Some(preedit) = Preedit::from_protocol(text.unwrap_or_default(), begin, end) else {
            return false;
        };
        self.batch.preedit = preedit;
        true
    }

    /// 【输入法】【确认暂存】将 text 暂存为本批确认文字，空值按协议视为空字符串，无返回值
    pub fn commit(&mut self, text: Option<String>) {
        self.batch.commit = text.unwrap_or_default();
    }

    /// 【输入法】【删除暂存】暂存 before、after 的前后字节长度，替换同批较早请求，无返回值
    pub fn delete(&mut self, before: u32, after: u32) {
        self.batch.delete_before = before;
        self.batch.delete_after = after;
    }

    /// 【输入法】【批次交付】按 target_id、serial 与客户端 commit_count 返回完整更新；只有序号匹配允许同步，交付后重置全部事件状态
    pub fn done(&mut self, target_id: u64, serial: u32, commit_count: u32) -> Update {
        Update {
            target_id,
            serial,
            sync_allowed: serial == commit_count,
            batch: std::mem::take(&mut self.batch),
        }
    }

    /// 【输入法】【焦点重置】丢弃当前未交付批次，供协议 enable、disable 和 leave 边界调用，无参数、无返回值
    pub fn reset(&mut self) {
        self.batch = Batch::default();
    }
}
