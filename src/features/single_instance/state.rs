//! 【单实例】【截图调度】延迟请求互相替换，活动截图只确认命令，不叠加覆盖窗口

use super::request::{Plan, Request, response};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Waiting,
    Starting,
    Active,
    Closed,
}

pub(super) struct State {
    phase: Phase,
    plan: Plan,
    deferred: Option<Plan>,
}

impl State {
    /// 【单实例】【初始调度】按 plan 与 now 创建首个截图状态，未设置或已到期时立即保留启动职责，返回独立调度器
    pub fn new(plan: Plan, now: Instant) -> Self {
        Self {
            phase: if plan.deadline.is_some_and(|at| at > now) {
                Phase::Waiting
            } else {
                Phase::Starting
            },
            plan,
            deferred: None,
        }
    }

    /// 【单实例】【请求处理】按 request 和 now 更新待截图参数，返回回复及是否唤醒延迟等待；活动截图不创建新窗口
    pub fn request(&mut self, request: Request, now: Instant) -> (Vec<u8>, bool) {
        self.expire(now);
        let plan = match request.plan(now) {
            Ok(plan) => plan,
            Err(error) => return (response(false, error), false),
        };
        if self.phase == Phase::Closed {
            return (response(false, "capture instance is closing"), false);
        }
        if !request.capture {
            return (response(true, "running"), false);
        }
        // 1. 【单实例】【延迟替换】原版整个进程共用调度器，新请求覆盖尚未开始的截图参数与期限
        let changed = if self.phase == Phase::Waiting {
            self.plan = plan;
            true
        } else {
            // 2. 【单实例】【活动保护】定时到达时仍有截图窗口则跳过，窗口关闭后一次性进程直接退出
            self.deferred = plan.deadline.map(|_| plan);
            false
        };
        let message = if request.capture_delay_ms > 0 {
            format!("capture scheduled in {} ms", request.capture_delay_ms)
        } else {
            "capture requested".into()
        };
        (response(true, &message), changed)
    }

    /// 【单实例】【等待计划】无参数，返回仍可由新请求替换的截图计划，已开始或关闭时返回空值
    pub fn pending(&self) -> Option<Plan> {
        (self.phase == Phase::Waiting).then_some(self.plan)
    }

    /// 【单实例】【开始确认】按 now 原子确认最新计划是否到期，返回本次实际参数；未到期或已关闭返回空值
    pub fn begin(&mut self, now: Instant) -> Option<Plan> {
        if matches!(self.phase, Phase::Closed | Phase::Active)
            || self.phase == Phase::Waiting && self.plan.deadline.is_some_and(|at| at > now)
        {
            return None;
        }
        self.phase = Phase::Starting;
        Some(self.plan)
    }

    /// 【单实例】【活动确认】首帧准备完成后转为活动截图，无参数、无返回值，不改变已有延迟请求
    pub fn active(&mut self) {
        if self.phase != Phase::Closed {
            self.phase = Phase::Active;
        }
    }

    /// 【单实例】【下次期限】无参数，返回活动截图尚未到达的重复请求期限，普通空闲截图不创建计时器
    pub fn deadline(&self) -> Option<Instant> {
        self.deferred.and_then(|plan| plan.deadline)
    }

    /// 【单实例】【重复期限】按 now 清理已到期请求，活动截图保持原窗口，无返回值
    pub fn expire(&mut self, now: Instant) {
        if self.phase == Phase::Waiting
            && self.plan.deadline.is_some_and(|deadline| deadline <= now)
        {
            self.phase = Phase::Starting;
        }
        if self.deadline().is_some_and(|deadline| deadline <= now) {
            self.deferred = None;
        }
    }

    /// 【单实例】【退出确认】停止接收新截图职责并清理延迟，无参数、无返回值
    pub fn close(&mut self) {
        self.phase = Phase::Closed;
        self.deferred = None;
    }
}
