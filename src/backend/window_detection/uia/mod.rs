//! 【Windows控件识别】【查询契约】可测试的控件树与平台接口分离，原生对象由调用线程持有

#[cfg(test)]
mod tests;
mod tree;

use crate::features::region_detection::PixelRect;
use std::time::{Duration, Instant};
pub(crate) use tree::Tree;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stop {
    Complete,
    Budget,
    Decoding,
    Timeout,
    Failure,
    Cancelled,
    Limit,
}

impl Stop {
    /// 【Windows控件识别】【补充资格】无参数，返回当前中断是否允许鼠标停留后继续查询
    pub fn refinable(self) -> bool {
        matches!(self, Self::Budget | Self::Decoding | Self::Timeout)
    }
}

pub(crate) struct Candidate<E> {
    pub element: E,
    pub rect: PixelRect,
    pub offscreen: bool,
    pub structural: bool,
}

pub(crate) trait Batch {
    type Element;
    /// 【Windows控件识别】【批次大小】无参数，返回已经缓存的兄弟节点数量
    fn len(&self) -> usize;
    /// 【Windows控件识别】【批次解码】按index读取本地缓存节点，返回候选或该节点的读取错误
    fn get(&self, index: usize) -> Result<Candidate<Self::Element>, Stop>;
}

pub(crate) trait Provider {
    type Element;
    type Batch: Batch<Element = Self::Element>;
    /// 【Windows控件识别】【窗口根节点】按handle与limit读取窗口子节点，返回完整原生缓存批次或停止原因
    fn root(&mut self, handle: usize, limit: Duration) -> Result<Self::Batch, Stop>;
    /// 【Windows控件识别】【子节点读取】按element与limit读取其子节点，返回缓存批次或停止原因
    fn children(&mut self, element: &Self::Element, limit: Duration) -> Result<Self::Batch, Stop>;
}

pub(crate) trait Clock {
    /// 【Windows控件识别】【预算时钟】无参数，返回单调递增时间，测试可直接推进时间
    fn now(&self) -> Duration;
}

pub(crate) struct QueryClock(Instant);
impl QueryClock {
    /// 【Windows控件识别】【时钟创建】无参数，返回以当前时刻为原点的查询时钟
    pub fn new() -> Self {
        Self(Instant::now())
    }
}
impl Clock for QueryClock {
    /// 【Windows控件识别】【已用时间】无参数，返回本次查询已消耗的时间
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Policy {
    pub budget: Duration,
    pub call_limit: Duration,
    pub retry_timeout: bool,
    pub publish_interval: Option<Duration>,
}
impl Policy {
    /// 【Windows控件识别】【首次预算】无参数，返回短查询预算，首次查询不重试外部超时
    pub fn foreground() -> Self {
        Self {
            budget: Duration::from_millis(168),
            call_limit: Duration::from_millis(168),
            retry_timeout: false,
            publish_interval: None,
        }
    }
    /// 【Windows控件识别】【停留预算】无参数，返回补充查询预算及发布间隔，每个超时节点最多重试一次
    pub fn refinement() -> Self {
        Self {
            budget: Duration::from_millis(1500),
            call_limit: Duration::from_millis(500),
            retry_timeout: true,
            publish_interval: Some(Duration::from_millis(32)),
        }
    }
}

pub(crate) struct ResultPath {
    pub path: Vec<PixelRect>,
    pub stop: Stop,
}
