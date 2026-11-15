//! 【Windows控件测试】【确定性缓存】使用可推进时钟验证外部慢接口、重叠顺序和重复容器

use super::*;
use std::{cell::Cell, collections::HashMap, rc::Rc};

#[derive(Clone)]
struct Spec {
    id: usize,
    rect: PixelRect,
    structural: bool,
    offscreen: bool,
}
impl Spec {
    /// 【Windows控件测试】【节点样本】按id与rect返回可见实际控件
    fn new(id: usize, rect: PixelRect) -> Self {
        Self {
            id,
            rect,
            structural: false,
            offscreen: false,
        }
    }
    /// 【Windows控件测试】【结构样本】按id与rect返回Pane或Group样本
    fn pane(id: usize, rect: PixelRect) -> Self {
        Self {
            structural: true,
            ..Self::new(id, rect)
        }
    }
}
#[derive(Clone, Default)]
struct Time(Rc<Cell<Duration>>);
impl Clock for Time {
    /// 【Windows控件测试】【读取时钟】无参数，返回测试已经推进的时间
    fn now(&self) -> Duration {
        self.0.get()
    }
}
struct FakeBatch {
    items: Vec<Spec>,
    time: Time,
    cost: Duration,
}
impl Batch for FakeBatch {
    type Element = usize;
    /// 【Windows控件测试】【样本数量】无参数，返回兄弟样本数量
    fn len(&self) -> usize {
        self.items.len()
    }
    /// 【Windows控件测试】【样本解码】按index读取候选并推进时钟，返回对应样本
    fn get(&self, index: usize) -> Result<Candidate<usize>, Stop> {
        self.time.0.set(self.time.now() + self.cost);
        let spec = &self.items[index];
        Ok(Candidate {
            element: spec.id,
            rect: spec.rect,
            structural: spec.structural,
            offscreen: spec.offscreen,
        })
    }
}
#[derive(Default)]
struct Fake {
    children: HashMap<usize, Vec<Spec>>,
    calls: Vec<usize>,
    limits: Vec<Duration>,
    time: Time,
    acquisition: Duration,
    decoding: Duration,
    failure: Option<(usize, Stop)>,
}
impl Fake {
    /// 【Windows控件测试】【批次读取】按id与limit记录实际访问，返回缓存样本或指定错误
    fn load(&mut self, id: usize, limit: Duration) -> Result<FakeBatch, Stop> {
        self.calls.push(id);
        self.limits.push(limit);
        self.time.0.set(self.time.now() + self.acquisition);
        if let Some((failed, stop)) = self.failure
            && failed == id
        {
            return Err(stop);
        }
        Ok(FakeBatch {
            items: self.children.get(&id).cloned().unwrap_or_default(),
            time: self.time.clone(),
            cost: self.decoding,
        })
    }
}
impl Provider for Fake {
    type Element = usize;
    type Batch = FakeBatch;
    /// 【Windows控件测试】【根节点读取】按handle与limit读取样本根，返回批次
    fn root(&mut self, handle: usize, limit: Duration) -> Result<FakeBatch, Stop> {
        self.load(handle, limit)
    }
    /// 【Windows控件测试】【后代读取】按element与limit读取样本后代，返回批次
    fn children(&mut self, element: &usize, limit: Duration) -> Result<FakeBatch, Stop> {
        self.load(*element, limit)
    }
}
/// 【Windows控件测试】【窗口样本】无参数，返回包含全部测试控件的窗口
fn bounds() -> PixelRect {
    PixelRect::new(-100, -100, 400, 400)
}
/// 【Windows控件测试】【查询样本】按tree、provider与policy命中固定位置，返回真实算法结果
fn hit(tree: &mut Tree<Fake>, provider: &mut Fake, policy: Policy) -> ResultPath {
    let clock = provider.time.clone();
    tree.query(provider, (10, 10), policy, &clock, &|| false, &mut |_| {})
}

/// 【Windows控件测试】【真实层级】无参数，验证最深组件优先、窗口末尾和重复缓存不访问外部接口
#[test]
fn deepest_path_reuses_complete_batches() {
    let pane = PixelRect::new(0, 0, 100, 100);
    let leaf = PixelRect::new(5, 5, 20, 20);
    let mut provider = Fake::default();
    provider.children.insert(
        0,
        vec![
            Spec::new(1, pane),
            Spec::new(2, PixelRect::new(150, 0, 50, 50)),
        ],
    );
    provider.children.insert(1, vec![Spec::new(3, leaf)]);
    let mut tree = Tree::new(0, bounds());
    for _ in 0..100 {
        assert_eq!(
            hit(&mut tree, &mut provider, Policy::foreground()).path,
            [leaf, pane, bounds()]
        );
    }
    assert_eq!(provider.calls, [0, 1, 3]);
}

/// 【Windows控件测试】【重叠顺序】无参数，验证后出现的兄弟优先，实际控件不得因更小的后代被越过
#[test]
fn last_overlapping_control_keeps_precedence() {
    let mut provider = Fake::default();
    let small = PixelRect::new(5, 5, 20, 20);
    provider
        .children
        .insert(0, vec![Spec::pane(1, bounds()), Spec::new(2, bounds())]);
    provider.children.insert(1, vec![Spec::new(3, small)]);
    assert_eq!(
        hit(
            &mut Tree::new(0, bounds()),
            &mut provider,
            Policy::foreground()
        )
        .path,
        [bounds()]
    );
    assert_eq!(provider.calls, [0, 2]);
}

/// 【Windows控件测试】【浏览器结构】无参数，验证同边界重复结构容器不会遮蔽内容分支
#[test]
fn redundant_panes_backtrack_to_content_without_requerying() {
    let leaf = PixelRect::new(5, 5, 20, 20);
    let mut provider = Fake::default();
    provider
        .children
        .insert(0, vec![Spec::pane(1, bounds()), Spec::pane(2, bounds())]);
    provider.children.insert(2, vec![Spec::pane(3, bounds())]);
    provider.children.insert(1, vec![Spec::new(4, leaf)]);
    let mut tree = Tree::new(0, bounds());
    assert_eq!(
        hit(&mut tree, &mut provider, Policy::foreground()).path,
        [leaf, bounds()]
    );
    let calls = provider.calls.clone();
    assert_eq!(
        hit(&mut tree, &mut provider, Policy::foreground()).path,
        [leaf, bounds()]
    );
    assert_eq!(provider.calls, calls);
}

/// 【Windows控件测试】【不完整兄弟】无参数，验证解码可续接，首次预算结束不发布未确认的重叠控件
#[test]
fn partial_batch_resumes_without_refetching_or_premature_overlap() {
    let leaf = PixelRect::new(5, 5, 20, 20);
    let mut provider = Fake {
        decoding: Duration::from_millis(100),
        ..Fake::default()
    };
    provider.children.insert(
        0,
        vec![
            Spec::new(1, bounds()),
            Spec::new(2, bounds()),
            Spec::new(3, leaf),
        ],
    );
    let mut tree = Tree::new(0, bounds());
    let initial = hit(&mut tree, &mut provider, Policy::foreground());
    assert_eq!(initial.path, [bounds()]);
    assert_eq!(initial.stop, Stop::Decoding);
    assert!(initial.stop.refinable());
    assert_eq!(
        hit(&mut tree, &mut provider, Policy::refinement()).path,
        [leaf, bounds()]
    );
    assert_eq!(provider.calls, [0, 3]);
}

/// 【Windows控件测试】【超时回退】无参数，验证超时只在补充查询重试一次，最终始终保留窗口
#[test]
fn provider_timeout_has_one_refinement_retry() {
    let mut provider = Fake {
        failure: Some((0, Stop::Timeout)),
        ..Fake::default()
    };
    let mut tree = Tree::new(0, bounds());
    for _ in 0..3 {
        assert_eq!(
            hit(&mut tree, &mut provider, Policy::foreground()).path,
            [bounds()]
        );
    }
    assert_eq!(provider.calls, [0]);
    for _ in 0..3 {
        assert_eq!(
            hit(&mut tree, &mut provider, Policy::refinement()).stop,
            Stop::Timeout
        );
    }
    assert_eq!(provider.calls, [0, 0]);
}

/// 【Windows控件测试】【重试错误】无参数，验证补充重试发现永久失败后不再把该节点当成可重试超时
#[test]
fn permanent_failure_after_timeout_is_cached_as_failure() {
    let mut provider = Fake {
        failure: Some((0, Stop::Timeout)),
        ..Fake::default()
    };
    let mut tree = Tree::new(0, bounds());
    assert_eq!(
        hit(&mut tree, &mut provider, Policy::foreground()).stop,
        Stop::Timeout
    );
    provider.failure = Some((0, Stop::Failure));
    assert_eq!(
        hit(&mut tree, &mut provider, Policy::refinement()).stop,
        Stop::Failure
    );
    assert_eq!(
        hit(&mut tree, &mut provider, Policy::foreground()).stop,
        Stop::Failure
    );
    assert_eq!(provider.calls, [0, 0]);
}

/// 【Windows控件测试】【中断路径】无参数，验证接口失败保留最佳已确认层级且不重复调用失败节点
#[test]
fn provider_failure_keeps_known_path() {
    let pane = PixelRect::new(0, 0, 100, 100);
    let mut provider = Fake {
        failure: Some((1, Stop::Failure)),
        ..Fake::default()
    };
    provider.children.insert(0, vec![Spec::new(1, pane)]);
    let mut tree = Tree::new(0, bounds());
    for _ in 0..3 {
        assert_eq!(
            hit(&mut tree, &mut provider, Policy::foreground()).path,
            [pane, bounds()]
        );
    }
    assert_eq!(provider.calls, [0, 1]);
}

/// 【Windows控件测试】【几何校验】无参数，验证离屏、无交集节点忽略，子节点按窗口裁切并允许超出父容器
#[test]
fn clipping_uses_window_not_parent_geometry() {
    let mut provider = Fake::default();
    let pane = PixelRect::new(5, 5, 20, 20);
    let wide = PixelRect::new(-200, 0, 500, 40);
    provider.children.insert(0, vec![Spec::new(1, pane)]);
    provider.children.insert(
        1,
        vec![
            Spec::new(2, wide),
            Spec {
                offscreen: true,
                ..Spec::new(3, pane)
            },
            Spec::new(4, PixelRect::new(500, 500, 10, 10)),
        ],
    );
    assert_eq!(
        hit(
            &mut Tree::new(0, bounds()),
            &mut provider,
            Policy::foreground()
        )
        .path,
        [wide.intersect(bounds()).unwrap(), pane, bounds()]
    );
}

/// 【Windows控件测试】【过期取消】无参数，验证取消及窗口外命中不调用外部服务
#[test]
fn cancellation_and_outside_hits_do_not_call_provider() {
    let mut provider = Fake::default();
    let clock = QueryClock::new();
    let mut tree = Tree::new(0, bounds());
    assert_eq!(
        tree.query(
            &mut provider,
            (10, 10),
            Policy::foreground(),
            &clock,
            &|| true,
            &mut |_| {}
        )
        .stop,
        Stop::Cancelled
    );
    assert!(
        tree.query(
            &mut provider,
            (300, 10),
            Policy::foreground(),
            &clock,
            &|| false,
            &mut |_| {}
        )
        .path
        .is_empty()
    );
    assert!(provider.calls.is_empty());
}
