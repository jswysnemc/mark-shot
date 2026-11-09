//! 【截图控件识别】【输入接入】保留精确控件层级，异步结果只更新未按下的当前指针

use crate::{
    backend::{
        task_wake::Wake,
        window_detection::{Window, windows::Worker},
    },
    editor::EditorState,
    features::region_detection::PixelRect,
};
use tiny_skia::Rect;

pub(super) struct Native {
    worker: Worker,
    point: Option<(i32, i32)>,
    path: Vec<PixelRect>,
    level: usize,
    windows: Vec<Window>,
    window: Option<usize>,
    anchor: Option<(i32, i32)>,
}

impl Native {
    /// 【截图控件识别】【会话创建】按windows与wake启动后台服务，返回候选状态或线程错误
    pub fn new(windows: &[Window], wake: Wake, bounds: Rect) -> std::io::Result<Self> {
        let windows: Vec<_> = windows
            .iter()
            .filter_map(|window| {
                Some(Window {
                    rect: window.rect.intersect(&bounds)?,
                    ..*window
                })
            })
            .collect();
        Ok(Self {
            worker: Worker::new(windows.clone(), wake)?,
            point: None,
            path: Vec::new(),
            level: 0,
            windows,
            window: None,
            anchor: None,
        })
    }
    /// 【截图控件识别】【指针请求】按point更新物理位置，保留仍命中的缓存叶子，返回当前可用控件范围
    pub fn hover(&mut self, point: (f64, f64)) -> Option<Rect> {
        let point = (point.0.floor() as i32, point.1.floor() as i32);
        if self.point != Some(point) {
            let window = self
                .windows
                .iter()
                .find(|window| {
                    point.0 as f32 >= window.rect.left()
                        && (point.0 as f32) < window.rect.right()
                        && point.1 as f32 >= window.rect.top()
                        && (point.1 as f32) < window.rect.bottom()
                })
                .map(|window| window.native_id);
            let nearby = self.anchor.is_some_and(|previous| {
                (f64::from(previous.0) - f64::from(point.0))
                    .hypot(f64::from(previous.1) - f64::from(point.1))
                    <= 6.0
            });
            if self.window != window || !self.path.first().is_some_and(|rect| rect.contains(point))
            {
                self.path.clear();
                self.level = 0;
            } else if !nearby {
                self.level = 0;
            }
            self.point = Some(point);
            self.window = window;
            self.worker.request(point);
        }
        self.current()
    }
    /// 【截图控件识别】【当前层级】无参数，返回当前控件的物理矩形，单独窗口留给图像和窗口回退处理
    fn current(&self) -> Option<Rect> {
        if self.path.len() < 2 {
            return None;
        }
        let rect = self.path.get(self.level)?;
        Rect::from_xywh(
            rect.left as f32,
            rect.top as f32,
            rect.width() as f32,
            rect.height() as f32,
        )
    }
    /// 【截图控件识别】【滚轮选择】按direction扩大或缩小真实控件层级，返回新范围，没有控件时返回空值
    pub fn step(&mut self, direction: f64) -> Option<Rect> {
        self.anchor = self.point;
        if direction > 0.0 {
            self.level = (self.level + 1).min(self.path.len().saturating_sub(1));
        } else if direction < 0.0 {
            self.level = self.level.saturating_sub(1);
        }
        self.current()
    }
}

/// 【截图控件识别】【结果接入】按state接收最新控件路径，dirty记录重绘，按下期间不改变已经锁定的点击候选
pub(super) fn poll(state: &mut EditorState, dirty: &mut u32) {
    let region = &mut state.region_detection;
    let Some(native) = &mut region.native else {
        return;
    };
    let Some(hit) = native.worker.take() else {
        return;
    };
    if native.point != Some(hit.point) || region.pressed.is_some() || state.input.mouse_down {
        return;
    }
    let previous = native.path.get(native.level).copied();
    let manual = native.level > 0;
    if let (Some(current), Some(next)) = (native.path.first(), hit.path.first())
        && current != next
        && next.contains_rect(*current)
        && native.path.last() == hit.path.last()
    {
        return;
    }
    native.path = hit.path;
    native.level = if manual {
        previous
            .and_then(|rect| native.path.iter().position(|candidate| *candidate == rect))
            .unwrap_or(0)
    } else {
        0
    };
    if region.pointer_valid {
        super::input::refresh(state, dirty);
    }
}
