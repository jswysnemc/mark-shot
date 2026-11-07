//! 【滚动截图】【完成事务】等待已经在途的帧后交付原图，等待期间保留事件循环与立即取消

use super::{PendingOutcome as Outcome, ScrollRun, handoff};
use crate::{backend::ScreenOverlay, editor::EditorState, scroll::panel::PanelButton};

#[derive(Default)]
pub(super) struct Completion {
    pending: Option<Request>,
}

struct Request {
    button: PanelButton,
    paused: bool,
}

impl Completion {
    /// 【滚动截图】【完成请求】记录 button 和 run 原暂停状态并停止后续取帧，无返回值，不丢弃已在途结果
    pub(super) fn request(&mut self, button: PanelButton, run: &mut ScrollRun) {
        let paused = self
            .pending
            .as_ref()
            .map_or(run.session.paused, |request| request.paused);
        self.pending = Some(Request { button, paused });
        run.session.set_paused(true);
        run.dirty = true;
    }

    /// 【滚动截图】【完成撤销】移除尚未交付的操作，无参数、无返回值，恢复捕获由调用方处理
    pub(super) fn cancel(&mut self) {
        self.pending = None;
    }

    /// 【滚动截图】【完成交付】busy 表示仍有在途帧；空闲后用 state、overlay 和 run 交付原图或恢复取消保存
    pub(super) fn finish(
        &mut self,
        busy: bool,
        state: &EditorState,
        overlay: &mut dyn ScreenOverlay,
        run: &mut ScrollRun,
    ) -> crate::AppResult<Option<Outcome>> {
        if busy {
            return Ok(None);
        }
        let Some(request) = self.pending.take() else {
            return Ok(None);
        };
        match request.button {
            PanelButton::Annotate => handoff::annotate(run).map(Some),
            PanelButton::Copy => handoff::copy(run).map(Some),
            PanelButton::Save => {
                if let Some(path) = handoff::save(run, overlay, state.output.as_deref())? {
                    return Ok(Some(Outcome::Saved(path)));
                }
                // 1. 【滚动截图】【保存恢复】恢复发起保存前的状态，捕获错误仍保持暂停
                if run.session.status != "Capture error" {
                    run.session.set_paused(request.paused);
                }
                run.dirty = true;
                Ok(None)
            }
            _ => Err("invalid scroll completion action".into()),
        }
    }
}
