//! 【工具提示】【延迟状态】只在悬停目标稳定后显示，稳定提示不安排周期帧

use std::time::{Duration, Instant};
use tiny_skia::{Pixmap, Rect};

pub(crate) const DELAY: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Target {
    pub button: crate::ui::toolbar::ToolbarButton,
    pub secondary: bool,
    pub monitor_idx: usize,
    pub cell: Rect,
    pub toolbar: Rect,
    pub vertical: bool,
}

pub(crate) struct View {
    pub text: String,
    pub rect: Rect,
    pub monitor_idx: usize,
    pub cache: Option<Pixmap>,
    pub theme: Option<crate::theme::Mode>,
}

#[derive(Default)]
pub struct State {
    tracking: bool,
    target: Option<Target>,
    deadline: Option<Instant>,
    pub(crate) view: Option<View>,
}

impl State {
    /// 【工具提示】【悬停更新】按 target 与 now 更新稳定期限，返回需要恢复底图的旧提示范围；同一按钮内移动不重新计时
    pub(crate) fn hover(&mut self, target: Option<Target>, now: Instant) -> Option<(usize, Rect)> {
        self.tracking = true;
        if self.target == target {
            return None;
        }
        // 1. 【工具提示】【目标变更】按钮、显示器或实际布局变化后重新等待，旧提示立即撤下
        self.target = target;
        self.deadline = target.and_then(|_| now.checked_add(DELAY));
        self.view.take().map(|view| (view.monitor_idx, view.rect))
    }

    /// 【工具提示】【关闭】无参数，取消目标和期限，返回需要恢复底图的旧提示范围
    pub(crate) fn close(&mut self) -> Option<(usize, Rect)> {
        self.tracking = false;
        self.target = None;
        self.deadline = None;
        self.view.take().map(|view| (view.monitor_idx, view.rect))
    }

    /// 【工具提示】【当前目标】无参数，返回仍在等待或已显示的悬停目标
    pub(crate) fn target(&self) -> Option<Target> {
        self.target
    }

    /// 【工具提示】【轨迹资格】无参数，返回最近鼠标移动是否仍允许跟踪；点击或失焦后等待新的移动
    pub(crate) fn tracking(&self) -> bool {
        self.tracking
    }

    /// 【工具提示】【唤醒期限】无参数，返回唯一待显示期限；显示或关闭后返回空值
    pub(crate) fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    /// 【工具提示】【到期检查】按 now 判断是否已经达到稳定悬停期限，返回是否需要创建提示
    pub(crate) fn due(&self, now: Instant) -> bool {
        self.deadline.is_some_and(|deadline| now >= deadline)
    }

    /// 【工具提示】【显示】以 view 保存已完成布局的提示，无返回值；消费期限以停止定时唤醒
    pub(crate) fn show(&mut self, view: View) {
        self.view = Some(view);
        self.deadline = None;
    }
}
