//! 【截图控件识别】【输入接入】原生窗口归属与层级状态分离，后台结果只更新未按下的当前指针

use super::hit_path::HitPath;
#[cfg(target_os = "windows")]
use crate::backend::{
    task_wake::Wake,
    window_detection::{Window, windows::Worker},
};
use crate::{
    backend::window_detection::requests::Hit, editor::EditorState,
    features::region_detection::PixelRect,
};
use tiny_skia::Rect;

pub(super) struct Native {
    #[cfg(target_os = "windows")]
    worker: Option<Worker>,
    windows: Vec<(usize, PixelRect)>,
    path: HitPath,
}
impl Native {
    /// 【截图控件识别】【会话创建】按windows、wake与bounds启动后台服务，返回候选状态或线程错误
    #[cfg(target_os = "windows")]
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
        let worker = Worker::new(windows.clone(), wake)?;
        Ok(Self {
            worker: Some(worker),
            windows: windows
                .iter()
                .map(|window| {
                    (
                        window.native_id,
                        PixelRect::new(
                            window.rect.left() as i32,
                            window.rect.top() as i32,
                            window.rect.width() as i32,
                            window.rect.height() as i32,
                        ),
                    )
                })
                .collect(),
            path: HitPath::default(),
        })
    }
    /// 【截图控件测试】【冻结样本】按windows构造不调用系统的输入状态，返回可接入真实编辑器分派的样本
    #[cfg(test)]
    pub(super) fn fixture(windows: Vec<(usize, PixelRect)>) -> Self {
        Self {
            windows,
            path: HitPath::default(),
            #[cfg(target_os = "windows")]
            worker: None,
        }
    }
    /// 【截图控件识别】【指针请求】按point命中冻结前窗，返回当前范围，普通模式请求组件，整窗模式撤销组件查询
    pub fn hover(&mut self, point: (f64, f64)) -> Option<Rect> {
        if !point.0.is_finite() || !point.1.is_finite() {
            return None;
        }
        let point = (point.0.floor() as i32, point.1.floor() as i32);
        let window = self
            .windows
            .iter()
            .copied()
            .find(|(_, rect)| rect.contains(point));
        if self.path.target(point, window) {
            self.request();
        }
        self.current()
    }
    /// 【截图控件识别】【查询交付】无参数，把当前组件目标发送到工作线程，整窗模式仅取消，无返回值
    fn request(&self) {
        #[cfg(target_os = "windows")]
        if let Some(worker) = &self.worker {
            if self.path.window_only() {
                worker.cancel();
            } else if let Some(point) = self.path.point() {
                worker.request(point);
            }
        }
    }
    /// 【截图控件识别】【当前范围】无参数，返回组件或显式整窗物理矩形，单一默认窗口交给图像回退
    fn current(&self) -> Option<Rect> {
        if !self.path.native_selection() {
            return None;
        }
        let rect = self.path.current()?;
        Rect::from_xywh(
            rect.left as f32,
            rect.top as f32,
            rect.width() as f32,
            rect.height() as f32,
        )
    }
    /// 【截图控件识别】【滚轮选择】按direction累积真实控件层级，返回当前范围，保留显式选择身份
    pub fn step(&mut self, direction: f64) -> Option<Rect> {
        if !self.path.native_selection() && direction <= 0.0 {
            return None;
        }
        self.path.step(direction);
        self.current()
    }
    /// 【截图控件识别】【整窗切换】无参数，切换目标并撤销或重新请求组件，返回当前可用范围
    pub fn toggle(&mut self) -> Option<Rect> {
        self.path.toggle();
        self.request();
        self.current()
    }
}

/// 【截图控件识别】【结果接入】按state与hit接入当前路径，dirty记录重绘，按下期间不改变锁定候选
pub(super) fn apply(state: &mut EditorState, hit: Hit, dirty: &mut u32) {
    let region = &mut state.region_detection;
    if region.pressed.is_some() || state.input.mouse_down {
        return;
    }
    let Some(native) = &mut region.native else {
        return;
    };
    if native.path.apply(hit.point, hit.path, hit.refined) && region.pointer_valid {
        super::input::refresh(state, dirty);
    }
}
/// 【截图控件识别】【完成接收】按state读取唯一最新结果并接入路径，dirty返回需要重绘的区域，无持续轮询
#[cfg(target_os = "windows")]
pub(super) fn poll(state: &mut EditorState, dirty: &mut u32) {
    let hit = state
        .region_detection
        .native
        .as_ref()
        .and_then(|native| native.worker.as_ref())
        .and_then(Worker::take);
    if let Some(hit) = hit {
        apply(state, hit, dirty);
    }
}
