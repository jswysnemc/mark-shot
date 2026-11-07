//! 【截图区域识别】【一次性分析】共享冻结像素，完成后通知并回收工作线程

use super::{geometry::Space, input, state::State};
use crate::{
    backend::ScreenOverlay,
    editor::EditorState,
    features::region_detection::Detector,
    types::{Capture, Placement},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
};

pub(super) struct Worker {
    result: mpsc::Receiver<Result<Detector, String>>,
    thread: Option<JoinHandle<()>>,
    cancelled: Arc<AtomicBool>,
}

impl Drop for Worker {
    /// 【截图区域识别】【任务回收】无参数、无返回值，取消未开始分析并回收唯一线程，不保留后台线程池
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// 【截图区域识别】【冻结分析】按 captures、placements 和 space 分析原图，返回检测器；单输出直接共享像素，多输出合成一次
fn detect(
    captures: &[Capture],
    placements: &[Placement],
    space: Space,
) -> Result<Detector, String> {
    let shared;
    let combined;
    let pixels = if captures.len() == 1 {
        shared = captures[0].pixmap.share();
        shared.as_ref().as_ref()
    } else {
        combined = crate::renderer::composite(
            captures,
            placements,
            space.bounds,
            Some(space.scale as f32),
        )
        .ok_or("region detection could not compose frozen outputs")?
        .0;
        combined.as_ref()
    };
    let result = Detector::from_rgba(
        pixels.width() as i32,
        pixels.height() as i32,
        pixels.data(),
        true,
    )
    .map_err(str::to_owned);
    // 1. 【截图区域识别】【读取结束】分析已经生成独立检测数据，归还共享原件页，不提前删除后台来源
    for capture in captures {
        capture.pixmap.release_pages();
    }
    result
}

/// 【截图区域识别】【会话准备】按 state、overlay 与 windows 初始化独立开关，返回值保存在 state；仅图像分析需要完成通知
pub(crate) fn start(
    state: &mut EditorState,
    overlay: &mut dyn ScreenOverlay,
    windows: &[crate::backend::window_detection::Window],
) {
    let Some(space) = Space::for_editor(state) else {
        return;
    };
    let config = crate::config::get();
    if !config.region_detection.enabled && !config.window_detection.enabled {
        return;
    }
    state.region_detection = State {
        space: Some(space),
        windows: if config.window_detection.enabled {
            space.windows(windows)
        } else {
            Vec::new()
        },
        ..State::default()
    };
    if !config.region_detection.enabled || space.size.0 < 3 || space.size.1 < 3 {
        return;
    }
    let wake = match overlay.task_wake() {
        Ok(wake) => wake,
        Err(error) => {
            log::info!("【截图区域识别】【会话准备】完成通知不可用：{error}");
            return;
        }
    };
    let captures = state.captures.clone();
    let placements = state.placements.clone();
    let (sender, result) = mpsc::channel();
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    // 1. 【截图区域识别】【原图共享】任务持有共享原图和轻量布局，主界面无需等待分析完成
    let thread = std::thread::spawn(move || {
        let result = if flag.load(Ordering::Relaxed) {
            Err("region detection cancelled".into())
        } else {
            std::panic::catch_unwind(|| detect(&captures, &placements, space))
                .unwrap_or_else(|_| Err("region detection worker panicked".into()))
        };
        let _ = sender.send(result);
        wake.notify();
    });
    state.region_detection.worker = Some(Worker {
        result,
        thread: Some(thread),
        cancelled,
    });
}

/// 【截图区域识别】【完成接入】按 state 接收唯一后台结果并恢复当前悬停，dirty 返回需要重绘的显示器，无持续轮询
pub(crate) fn poll(state: &mut EditorState, dirty: &mut u32) {
    let Some(worker) = state.region_detection.worker.take() else {
        return;
    };
    let result = match worker.result.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => {
            state.region_detection.worker = Some(worker);
            return;
        }
        Err(mpsc::TryRecvError::Disconnected) => Err("region detection worker disconnected".into()),
    };
    drop(worker);
    match result {
        Ok(mut detector) => {
            let windows: Vec<_> = state
                .region_detection
                .windows
                .iter()
                .map(|window| window.rect)
                .collect();
            detector.add_windows(&windows);
            log::info!(
                "【截图区域识别】【完成接入】候选数量 {}",
                detector.candidates().len()
            );
            state.region_detection.detector = Some(detector);
            state.region_detection.cursor.reset();
            if state.region_detection.pressed.is_none() && state.region_detection.pointer_valid {
                input::refresh(state, dirty);
            }
        }
        Err(error) => log::error!("【截图区域识别】【完成接入】{error}"),
    }
}
