//! 【Windows控件识别】【缓存树】完整兄弟批次决定命中顺序，重复结构容器允许回退到内容分支

use super::{Batch, Clock, Policy, Provider, ResultPath, Stop};
use crate::features::region_detection::PixelRect;
use std::time::Duration;

const MAX_STEPS: usize = 80;
const MAX_NODES: usize = 32768;
const MAX_CHILDREN: usize = 8192;

enum Children<E, B> {
    Root(usize),
    Unloaded(E),
    Pending {
        batch: B,
        next: usize,
        nodes: Vec<usize>,
    },
    Loaded(Vec<usize>),
    Failed(Stop),
}

struct Node<E, B> {
    rect: PixelRect,
    parent: Option<usize>,
    structural: bool,
    retried: bool,
    timed_out: bool,
    children: Children<E, B>,
}

pub(crate) struct Tree<P: Provider> {
    nodes: Vec<Node<P::Element, P::Batch>>,
}

impl<P: Provider> Tree<P> {
    /// 【Windows控件识别】【树创建】按handle与bounds建立冻结窗口根节点，返回尚未访问外部控件的缓存
    pub fn new(handle: usize, bounds: PixelRect) -> Self {
        Self {
            nodes: vec![Node {
                rect: bounds,
                parent: None,
                structural: false,
                retried: false,
                timed_out: false,
                children: Children::Root(handle),
            }],
        }
    }

    /// 【Windows控件识别】【路径查询】按provider、point、policy与clock查询，cancelled终止旧请求，publish接收补充路径，返回可靠路径与停止原因
    pub fn query(
        &mut self,
        provider: &mut P,
        point: (i32, i32),
        policy: Policy,
        clock: &impl Clock,
        cancelled: &impl Fn() -> bool,
        publish: &mut impl FnMut(Vec<PixelRect>),
    ) -> ResultPath {
        if !self.nodes[0].rect.contains(point) {
            return ResultPath {
                path: Vec::new(),
                stop: Stop::Complete,
            };
        }
        let deadline = clock.now() + policy.budget;
        let mut last_publish = clock.now();
        let mut current = 0;
        let mut stop = Stop::Limit;
        // 1. 【Windows控件识别】【路径展开】只沿鼠标命中的分支展开，已有完整缓存不再调用外部接口
        for _ in 0..MAX_STEPS {
            if cancelled() {
                stop = Stop::Cancelled;
                break;
            }
            if let Err(reason) = self.expand(current, provider, deadline, policy, clock, cancelled)
            {
                stop = reason;
                break;
            }
            let Some(child) = self.hit(current, point, usize::MAX) else {
                if let Some(alternative) = self.alternative(current, point) {
                    current = alternative;
                    continue;
                }
                stop = Stop::Complete;
                break;
            };
            current = child;
            if policy
                .publish_interval
                .is_some_and(|interval| clock.now() - last_publish >= interval)
                && !cancelled()
            {
                publish(self.path(current));
                last_publish = clock.now();
            }
        }
        ResultPath {
            path: self.path(current),
            stop,
        }
    }

    /// 【Windows控件识别】【兄弟命中】按parent、point与before选择最后出现的有效兄弟，返回节点索引
    fn hit(&self, parent: usize, point: (i32, i32), before: usize) -> Option<usize> {
        let Children::Loaded(children) = &self.nodes[parent].children else {
            return None;
        };
        children
            .iter()
            .rev()
            .copied()
            .find(|index| *index < before && self.nodes[*index].rect.contains(point))
    }

    /// 【Windows控件识别】【结构回退】按current与point越过等边界Pane或Group，返回其他命中分支，实际控件不回退
    fn alternative(&self, mut current: usize, point: (i32, i32)) -> Option<usize> {
        while let Some(parent) = self.nodes[current].parent {
            if !self.nodes[current].structural
                || self.nodes[current].rect != self.nodes[parent].rect
            {
                return None;
            }
            if let Some(sibling) = self.hit(parent, point, current) {
                return Some(sibling);
            }
            current = parent;
        }
        None
    }

    /// 【Windows控件识别】【路径生成】按current回溯真实父节点，返回去重后的最深组件至窗口路径
    fn path(&self, mut current: usize) -> Vec<PixelRect> {
        let bounds = self.nodes[0].rect;
        let mut path = Vec::new();
        while current != 0 && path.len() < MAX_STEPS {
            let node = &self.nodes[current];
            if node.rect != bounds && !path.contains(&node.rect) {
                path.push(node.rect);
            }
            current = node.parent.expect("child has parent");
        }
        path.push(bounds);
        path
    }

    /// 【Windows控件识别】【批次展开】按node、provider、deadline、policy、clock与cancelled继续缓存解码，返回完成状态或停止原因
    fn expand(
        &mut self,
        node: usize,
        provider: &mut P,
        deadline: Duration,
        policy: Policy,
        clock: &impl Clock,
        cancelled: &impl Fn() -> bool,
    ) -> Result<(), Stop> {
        if matches!(self.nodes[node].children, Children::Loaded(_)) {
            return Ok(());
        }
        let remaining = deadline.saturating_sub(clock.now()).min(policy.call_limit);
        if cancelled() {
            return Err(Stop::Cancelled);
        }
        if remaining.is_zero() {
            return Err(Stop::Budget);
        }
        if self.nodes[node].timed_out {
            if !policy.retry_timeout || self.nodes[node].retried {
                return Err(Stop::Timeout);
            }
            self.nodes[node].retried = true;
        }
        let state = std::mem::replace(
            &mut self.nodes[node].children,
            Children::Failed(Stop::Failure),
        );
        // 1. 【Windows控件识别】【外部获取】原生批次与未展开对象继续由同一线程持有，补充查询仅重试一次超时
        let acquired = match &state {
            Children::Root(handle) => Some(provider.root(*handle, remaining)),
            Children::Unloaded(element) => Some(provider.children(element, remaining)),
            _ => None,
        };
        let (batch, mut next, mut nodes) = match acquired {
            Some(Ok(batch)) => {
                self.nodes[node].timed_out = false;
                (batch, 0, Vec::new())
            }
            Some(Err(stop)) => {
                if stop != Stop::Timeout {
                    self.nodes[node].timed_out = false;
                }
                self.nodes[node].children = if stop == Stop::Timeout && !self.nodes[node].retried {
                    self.nodes[node].timed_out = true;
                    state
                } else {
                    Children::Failed(stop)
                };
                return Err(if cancelled() { Stop::Cancelled } else { stop });
            }
            None => match state {
                Children::Pending { batch, next, nodes } => (batch, next, nodes),
                Children::Failed(stop) => {
                    self.nodes[node].children = Children::Failed(stop);
                    return Err(stop);
                }
                _ => unreachable!(),
            },
        };
        if batch.len() > MAX_CHILDREN {
            self.nodes[node].children = Children::Failed(Stop::Limit);
            return Err(Stop::Limit);
        }
        // 2. 【Windows控件识别】【完整兄弟】保留未完成批次供下次继续，最后一个重叠兄弟尚未解码时不发布临时选择
        while next < batch.len() {
            if cancelled() || clock.now() >= deadline {
                self.nodes[node].children = Children::Pending { batch, next, nodes };
                return Err(if cancelled() {
                    Stop::Cancelled
                } else {
                    Stop::Decoding
                });
            }
            let candidate = batch.get(next);
            next += 1;
            let Ok(candidate) = candidate else { continue };
            if candidate.offscreen {
                continue;
            }
            let Some(rect) = candidate.rect.intersect(self.nodes[0].rect) else {
                continue;
            };
            if self.nodes.len() >= MAX_NODES {
                self.nodes[node].children = Children::Failed(Stop::Limit);
                return Err(Stop::Limit);
            }
            let index = self.nodes.len();
            self.nodes.push(Node {
                rect,
                parent: Some(node),
                structural: candidate.structural,
                retried: false,
                timed_out: false,
                children: Children::Unloaded(candidate.element),
            });
            nodes.push(index);
        }
        self.nodes[node].children = Children::Loaded(nodes);
        Ok(())
    }
}
