//! 【框选翻译】【编辑器接入】区域翻译状态与任务分离，退出先恢复原图再释放缓存

pub(super) mod handoff;
pub(super) mod input;
pub(super) mod panels;
mod recognition;
pub(super) mod tasks;

use crate::{
    editor::EditorState,
    features::selection_translation::session::Session,
    ui::selection_translation::{language_popup::Popup, result_bar::Bar},
};
use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) struct State {
    pub session: Option<Session>,
    pub draft: Option<Session>,
    pub popup: Option<Popup>,
    pub bar: Option<Bar>,
    pub requested: Option<String>,
    pub pending: Option<String>,
    pub task: Option<crate::translation::task::Task>,
    pub retired: Vec<crate::translation::task::Task>,
    pub ocr: Option<recognition::Task>,
    pub retired_ocr: Vec<recognition::Task>,
    pub recognizing: bool,
    pub background_dirty: bool,
    pub bar_pressed: bool,
    pub swallow_release: bool,
    pub ocr_deadline: Option<Instant>,
    pub ui_fonts: Option<cosmic_text::FontSystem>,
    pub ui_cache: Option<cosmic_text::SwashCache>,
}

impl State {
    /// 【框选翻译】【操作占用】返回是否有识别或翻译请求，无参数，旧任务回收不占用画布
    pub fn busy(&self) -> bool {
        self.requested.is_some() || self.pending.is_some()
    }

    /// 【框选翻译】【任务期限】返回实际工作或回收的短轮询间隔，无参数，空闲时没有定时唤醒
    pub fn poll_interval(&self) -> Option<Duration> {
        (!self.retired_ocr.is_empty() || !self.retired.is_empty())
            .then_some(Duration::from_millis(16))
    }
}

impl Drop for State {
    /// 【框选翻译】【窗口结束】取消识别并等待翻译线程真实回收，无参数，仅在事件循环结束后执行
    fn drop(&mut self) {
        if let Some(runtime) = &mut self.ocr {
            runtime.shutdown();
        }
        if let Some(task) = &mut self.task {
            task.shutdown();
        }
        for task in &mut self.retired {
            task.shutdown();
        }
        for task in &mut self.retired_ocr {
            task.shutdown();
        }
    }
}

/// 【框选翻译】【反馈提示】按 text 在 state 共用提示组件展示消息，dirty 返回重绘显示器
fn feedback(state: &mut EditorState, text: &str, dirty: &mut u32) {
    state.toasts.show_text(
        crate::ui::toast::ToastKind::TranslationFeedback,
        crate::ui::language::Language::current().text(text).into(),
        &mut state.text.font_system,
    );
    crate::editor::dirty::mark_dirty(dirty, state.input.pointer.monitor_idx);
}

/// 【框选翻译】【显示更新】把 state 当前语言或原图写回原始缓冲，dirty 返回重绘范围，不改变标注历史
pub(super) fn apply(state: &mut EditorState, dirty: &mut u32) {
    if let Some(session) = &state.selection_translation.session {
        let image = session
            .current()
            .filter(|_| session.showing_translation)
            .map(|t| &t.pixels);
        session
            .region
            .apply(&mut state.captures, state.file_canvas.as_mut(), image);
    }
    invalidate_background(state, dirty);
    panels::update(state, dirty);
    super::panels::toolbar::update_toolbar(state, dirty);
}

/// 【框选翻译】【模式开关】在 state 关闭语言列表、退出翻译或打开目标列表，dirty 返回面板范围
pub(super) fn toggle(state: &mut EditorState, dirty: &mut u32) {
    if state.selection_translation.popup.is_some() {
        panels::close(state, dirty);
    } else if state.selection_translation.session.is_some() {
        exit(state, dirty);
    } else {
        panels::open(state, false, dirty);
    }
}

/// 【框选翻译】【模式退出】取消 state 当前任务并逐字节恢复原图，dirty 返回全部输出，不删除标注
pub(super) fn exit(state: &mut EditorState, dirty: &mut u32) {
    tasks::cancel(state, dirty);
    panels::close(state, dirty);
    if let Some(session) = state.selection_translation.session.take() {
        session
            .region
            .apply(&mut state.captures, state.file_canvas.as_mut(), None);
        invalidate_background(state, dirty);
    }
    panels::update(state, dirty);
    super::panels::toolbar::update_toolbar(state, dirty);
}

/// 【框选翻译】【背景失效】原图像素改变后清理 state 的缩放缓存并登记完整输出，dirty 返回重绘掩码，避免复用旧译文背景
fn invalidate_background(state: &mut EditorState, dirty: &mut u32) {
    state.selection_translation.background_dirty = true;
    state.navigation.frames.clear();
    state.filters = Default::default();
    state.annotations_dirty = true;
    for monitor in 0..state.placements.len() {
        if let Some(rect) = state.placements[monitor].rect() {
            state.damage_local(monitor, rect);
        }
    }
    crate::editor::dirty::mark_all_dirty(dirty, state.placements.len());
}
