//! 【输入法】【请求规划】只在上下文变化时提交协议状态，停用和重新启用分别清理事件缓冲

use crate::{
    features::text::input_method::transaction::Pending,
    types::text_input::{ChangeCause, ContentType, Snapshot, Surrounding},
};
use std::collections::VecDeque;

#[derive(Debug, PartialEq, Eq)]
pub enum Request<'a> {
    Enable,
    Disable,
    Surrounding(&'a Surrounding),
    Cause(ChangeCause),
    ContentType(ContentType),
    Cursor((i32, i32, i32, i32)),
    Commit,
}

#[derive(Default)]
pub struct State {
    pub current: Option<Snapshot>,
    pub pending: Pending,
    pub commit_count: u32,
    epochs: VecDeque<(u32, Option<u64>)>,
    needs_disable: bool,
}

impl State {
    /// 【输入法】【状态请求】比较 snapshot 与当前状态并返回必要请求，空值停用；重新启用清除旧周边文字，序号按实际提交累计
    pub fn sync<'a>(&mut self, snapshot: Option<&'a Snapshot>) -> Vec<Request<'a>> {
        if self.current.as_ref() == snapshot {
            return Vec::new();
        }
        let mut requests = Vec::new();
        let Some(next) = snapshot else {
            if self.current.take().is_some() {
                self.pending.reset();
                self.needs_disable = false;
                requests.extend([Request::Disable, Request::Commit]);
                self.commit(None);
            }
            return requests;
        };
        let reset = self.current.as_ref().is_none_or(|old| {
            old.target_id != next.target_id
                || old.focus_id != next.focus_id
                || old.surrounding.is_some() != next.surrounding.is_some()
                || old.cursor_rect.is_some() != next.cursor_rect.is_some()
        });
        if reset {
            self.pending.reset();
            if self.current.is_some() || self.needs_disable {
                requests.extend([Request::Disable, Request::Commit]);
                self.commit(None);
            }
            requests.push(Request::Enable);
            self.needs_disable = false;
        }
        let old = self.current.as_ref();
        if reset || old.is_none_or(|old| old.content != next.content) {
            requests.push(Request::ContentType(next.content));
        }
        if (reset || old.is_none_or(|old| old.surrounding != next.surrounding))
            && let Some(context) = &next.surrounding
        {
            requests.push(Request::Surrounding(context));
        }
        requests.push(Request::Cause(next.cause));
        if (reset || old.is_none_or(|old| old.cursor_rect != next.cursor_rect))
            && let Some(rect) = next.cursor_rect
        {
            requests.push(Request::Cursor(rect));
        }
        requests.push(Request::Commit);
        self.commit(Some(next.target_id));
        self.current = Some(next.clone());
        requests
    }

    /// 【输入法】【原生重置】清理本地状态和未交付事件，保留提交计数及重新进入后的停用要求；leave 后服务端可能仍保留启用标志，无参数、无返回值
    pub fn reset(&mut self) {
        self.needs_disable |= self.current.is_some();
        self.current = None;
        self.pending.reset();
    }

    /// 【输入法】【提交计数】为 target 累计实际协议提交并记录字段边界，无返回值；只保存最近一百二十八个边界，不保存每帧记录
    fn commit(&mut self, target: Option<u64>) {
        self.commit_count = self.commit_count.wrapping_add(1);
        if self
            .epochs
            .back()
            .is_none_or(|(_, previous)| *previous != target)
        {
            self.epochs.push_back((self.commit_count, target));
            if self.epochs.len() > 128 {
                self.epochs.pop_front();
            }
        }
    }

    /// 【输入法】【序号目标】按 serial 返回它所知的原字段身份，停用序号或超出保留边界返回空值；使用协议整数回绕比较
    pub fn target_for(&self, serial: u32) -> Option<u64> {
        self.epochs
            .iter()
            .rev()
            .find(|(start, _)| serial.wrapping_sub(*start) < (1 << 31))
            .and_then(|(_, target)| *target)
    }
}
