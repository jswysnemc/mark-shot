//! 【截图窗口】【编辑器显隐】复用覆盖层挂起协议，隐藏期间保留冻结画布和标注

use crate::{
    backend::{ScreenOverlay, capture_visibility::Watch, task_wake::Wake},
    editor::EditorState,
};

pub(super) struct Visibility {
    wake: Wake,
    watch: Option<Watch>,
    hidden: bool,
}

impl Visibility {
    /// 【截图窗口】【编辑器登记】按 overlay 建立事件唤醒及窗口登记，返回可随截图会话释放的状态
    pub fn new(overlay: &mut dyn ScreenOverlay) -> crate::AppResult<Self> {
        let wake = overlay.task_wake()?;
        Ok(Self {
            wake,
            watch: None,
            hidden: false,
        })
    }

    /// 【截图窗口】【阻塞交接】无参数，在已挂起覆盖层即将交给嵌套窗口前注销本层，避免等待无法处理的确认
    pub fn pause(&mut self) {
        self.watch = None;
    }

    /// 【截图窗口】【显隐同步】按 state 与 overlay 应用隐藏请求，返回隐藏和恢复标记；恢复沿用原图，不重新捕获桌面
    pub fn sync(
        &mut self,
        state: &EditorState,
        overlay: &mut dyn ScreenOverlay,
    ) -> crate::AppResult<(bool, bool)> {
        if self.watch.is_none() {
            let wake = self.wake.clone();
            self.watch = Some(Watch::new(move || wake.notify())?);
        }
        let watch = self.watch.as_mut().unwrap();
        let requested = watch.requested()?;
        let restored = self.hidden && !requested;
        if requested != self.hidden {
            if requested {
                overlay.suspend()?;
            } else {
                overlay.resume()?;
                for (index, capture) in state.captures.iter().enumerate() {
                    overlay.set_background_source(index, &capture.pixmap)?;
                }
            }
            overlay.flush()?;
            self.hidden = requested;
        }
        watch.applied()?;
        Ok((self.hidden, restored))
    }
}
