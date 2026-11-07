//! 【图片上传】【编辑器衔接】主动上传最终选区，成功复制 URL 后关闭截图，失败不改变冻结图像或标注

use crate::{
    backend::ScreenOverlay,
    editor::EditorState,
    types::{CursorIcon, OverlayEvent},
    ui::toast::ToastKind,
};
use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) struct State {
    pub requested: bool,
    task: Option<crate::upload::task::Task>,
    close_at: Option<Instant>,
    canceled: bool,
    pub copied: bool,
}

impl State {
    /// 【图片上传】【忙碌资格】返回上传或成功延迟是否占用截图操作，无参数，不依赖动画或轮询状态
    pub fn busy(&self) -> bool {
        self.requested || self.task.is_some() || self.close_at.is_some()
    }

    /// 【图片上传】【完成期限】返回成功后关闭期限，无参数；后台上传自身没有周期计时要求
    pub fn deadline(&self) -> Option<Instant> {
        self.close_at
    }
}

/// 【图片上传】【开始请求】在 state 提交正文与属性，dirty 返回面板范围，无返回值；忙碌手势与无效选区不启动任务
pub(super) fn request(state: &mut EditorState, dirty: &mut u32) {
    if state.upload.busy()
        || state.tool_active
        || state.input.mouse_down
        || state.pending.is_some()
        || state.ann_drag.is_some()
        || !state
            .selection
            .zone
            .is_some_and(|region| region.width() >= 1.0 && region.height() >= 1.0)
    {
        return;
    }
    super::external_actions::close(state, dirty);
    crate::editor::text_session::commit(state, dirty);
    super::panels::color::commit_color_field_edit(state, dirty);
    super::panels::settings::commit_stepper_text_edit(state, dirty);
    state.upload.requested = true;
}

/// 【图片上传】【统一提示】把 text 加入 state 的现有 Toast，dirty 返回重绘范围，不展示响应正文和凭据
fn feedback(state: &mut EditorState, text: &str, dirty: &mut u32) {
    state.toasts.show_text(
        ToastKind::UploadFeedback,
        crate::ui::language::Language::current()
            .text(text)
            .to_owned(),
        &mut state.text.font_system,
    );
    crate::editor::dirty::mark_dirty(dirty, state.input.pointer.monitor_idx);
}

/// 【图片上传】【任务启动】从 state 渲染最终 PNG 并接入 overlay 通知，dirty 返回忙碌面板；失败保留原图和全部标注
pub(super) fn start(state: &mut EditorState, overlay: &mut dyn ScreenOverlay, dirty: &mut u32) {
    if !std::mem::take(&mut state.upload.requested) {
        return;
    }
    let result = (|| -> crate::AppResult<_> {
        let environment = std::env::vars_os()
            .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
            .collect();
        let config = crate::config::upload::current()
            .map_err(|_| "Invalid upload configuration")?
            .effective(&environment);
        let (png, _, _) = super::render_final(state).ok_or("Cannot render upload image")?;
        let png = crate::features::export_image::current(&png)?;
        let wake = overlay.task_wake()?;
        crate::upload::task::Task::start(config, png.into_owned(), move || wake.notify())
            .map_err(Into::into)
    })();
    match result {
        Ok(task) => {
            state.upload.canceled = false;
            state.upload.task = Some(task);
            overlay.set_cursor(CursorIcon::Progress);
            log::info!("【图片上传】【任务启动】开始上传当前选区");
        }
        Err(error) => {
            log::error!("【图片上传】【任务启动】{error}");
            feedback(state, "Upload failed", dirty);
        }
    }
    super::panels::toolbar::update_toolbar(state, dirty);
}

/// 【图片上传】【主动取消】取消 state 当前任务，dirty 返回提示范围；异步回收完成前继续阻止其他编辑，不退出截图
pub(super) fn cancel(state: &mut EditorState, dirty: &mut u32) {
    state.upload.requested = false;
    if let Some(task) = &mut state.upload.task {
        state.upload.canceled = true;
        task.cancel();
        feedback(state, "Upload canceled", dirty);
    }
}

/// 【图片上传】【输入隔离】在 state 上传期间消费 event，dirty 返回取消提示；焦点与修饰键继续由主循环处理，返回是否消费
pub(super) fn event(state: &mut EditorState, event: &OverlayEvent, dirty: &mut u32) -> bool {
    if !state.upload.busy() {
        return false;
    }
    match event {
        OverlayEvent::Key {
            chord: Some(chord), ..
        } if chord.key == crate::keys::Key::Escape => {
            cancel(state, dirty);
            true
        }
        OverlayEvent::Key { .. }
        | OverlayEvent::PointerMove { .. }
        | OverlayEvent::PointerButton { .. }
        | OverlayEvent::Scroll(_)
        | OverlayEvent::TextInput(_) => true,
        _ => false,
    }
}

/// 【图片上传】【完成处理】从 state 读取一次结果，dirty 返回工具栏和提示范围；复制失败恢复编辑，成功延迟一百五十毫秒关闭
pub(super) fn poll(state: &mut EditorState, dirty: &mut u32) {
    if state
        .upload
        .close_at
        .is_some_and(|deadline| Instant::now() >= deadline)
    {
        state.cancel = true;
        return;
    }
    let result = state
        .upload
        .task
        .as_mut()
        .and_then(crate::upload::task::Task::take_result);
    let Some(result) = result else {
        return;
    };
    state.upload.task.take().unwrap().shutdown();
    complete(state, result, dirty, crate::utils::copy_to_clipboard);
    if state.upload.canceled {
        log::info!("【图片上传】【取消完成】本次上传任务已回收");
    }
}

/// 【图片上传】【结果交付】按 result 和 copy 更新 state，dirty 返回恢复范围；复制失败或取消保留原截图，成功设置原版关闭期限
pub(super) fn complete(
    state: &mut EditorState,
    result: crate::upload::Result<crate::upload::Output>,
    dirty: &mut u32,
    copy: impl FnOnce(&str) -> crate::AppResult,
) {
    if !state.upload.canceled {
        match result {
            Ok(output) => match copy(&output.url) {
                Ok(()) => {
                    state.upload.copied = true;
                    state.upload.close_at = Some(Instant::now() + Duration::from_millis(150));
                    feedback(state, "Image URL copied", dirty);
                    log::info!("【图片上传】【任务完成】图片 URL 已复制");
                }
                Err(error) => {
                    log::error!("【图片上传】【结果复制】{error}");
                    feedback(state, "Copy failed", dirty);
                }
            },
            Err(error) => {
                log::error!("【图片上传】【任务失败】{error}");
                feedback(
                    state,
                    if error.kind == crate::upload::ErrorKind::Timeout {
                        "Upload timed out"
                    } else {
                        "Upload failed"
                    },
                    dirty,
                );
            }
        }
    }
    super::panels::toolbar::update_toolbar(state, dirty);
}
