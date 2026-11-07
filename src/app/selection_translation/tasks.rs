//! 【框选翻译】【异步任务】识别仅服务当前区域，服务完成主动唤醒，失败保持已显示内容

use super::{apply, feedback};
use crate::{
    AppResult,
    backend::ScreenOverlay,
    editor::EditorState,
    features::{
        image_translation::{layout, paint},
        selection_translation::{pixels::Region, session::Session},
    },
    ocr::OcrImage,
    translation::{request::Request, task::Task},
};
use std::time::{Duration, Instant};

/// 【框选翻译】【模型引导】在 overlay 释放输入后为 state 打开原生引导，返回模型与表面重建事实，取消不继续翻译
fn guide(
    state: &mut EditorState,
    overlay: &mut dyn ScreenOverlay,
) -> AppResult<Option<crate::ocr::models::ModelFiles>> {
    overlay.set_text_input(None)?;
    overlay.set_popup(None)?;
    overlay.suspend()?;
    let result = crate::features::ocr_model_guide::run();
    overlay.resume()?;
    for (index, capture) in state.captures.iter().enumerate() {
        overlay.set_background(
            index,
            capture.pixmap.data(),
            capture.pixmap.width(),
            capture.pixmap.height(),
        )?;
    }
    result
}

/// 【框选翻译】【请求启动】从 state 读取待选语言并接入 overlay 通知，dirty 返回显示范围，返回是否重建了表面
pub(in crate::app) fn start(
    state: &mut EditorState,
    overlay: &mut dyn ScreenOverlay,
    dirty: &mut u32,
) -> AppResult<bool> {
    let Some(target) = state.selection_translation.requested.take() else {
        return Ok(false);
    };
    // 1. 【框选翻译】【目标保存】保存失败仅报告日志，本次请求继续使用明确选择的语言
    if let Err(error) = crate::config::translation::save_target(&target) {
        log::error!("【框选翻译】【目标保存】{error}");
    }
    if state
        .selection_translation
        .session
        .as_mut()
        .is_some_and(|session| session.cached(&target))
    {
        apply(state, dirty);
        return Ok(false);
    }
    state.selection_translation.pending = Some(target);
    if state.selection_translation.session.is_some() {
        if let Err(error) = translate(state, overlay) {
            failed(state, error, dirty);
        }
        super::panels::update(state, dirty);
        return Ok(false);
    }
    let files = state
        .ocr
        .models
        .active()
        .and_then(|index| state.ocr.models.files(index));
    let rebuilt = files.is_none();
    let files = match files {
        Some(files) => Some(files),
        None => guide(state, overlay)?,
    };
    let Some(files) = files else {
        state.selection_translation.pending = None;
        super::panels::update(state, dirty);
        return Ok(rebuilt);
    };
    if rebuilt {
        state.ocr.models = crate::ocr::models::OcrModels::load();
    }
    // 2. 【框选翻译】【区域准备】识别使用带标注导出，取色与恢复使用尚未修改的冻结原图
    let prepared = (|| -> AppResult<_> {
        let region = state.selection.zone.ok_or("No text to translate")?;
        let original = Region::capture(state, region).ok_or("Cannot render translation image")?;
        let (png, _, _) =
            super::super::render_final(state).ok_or("Cannot render translation image")?;
        let pixels = tiny_skia::Pixmap::decode_png(&png)?;
        let mut rgb = Vec::with_capacity(pixels.width() as usize * pixels.height() as usize * 3);
        for pixel in pixels.data().as_chunks::<4>().0 {
            rgb.extend(pixel[..3].iter().map(|v| v.saturating_add(255 - pixel[3])));
        }
        let image = OcrImage {
            rgb,
            width: pixels.width(),
            height: pixels.height(),
            origin: (0.0, 0.0),
        };
        Ok((Session::new(original), image))
    })();
    match prepared {
        Ok((session, image)) => {
            match overlay
                .task_wake()
                .and_then(|wake| super::recognition::Task::start(image, files, wake))
            {
                Ok(task) => {
                    state.selection_translation.draft = Some(session);
                    state.selection_translation.ocr = Some(task);
                    state.selection_translation.recognizing = true;
                    state.selection_translation.ocr_deadline =
                        Some(Instant::now() + Duration::from_secs(30));
                }
                Err(error) => failed(state, error, dirty),
            }
        }
        Err(error) => failed(state, error, dirty),
    }
    super::panels::update(state, dirty);
    super::super::panels::toolbar::update_toolbar(state, dirty);
    Ok(rebuilt)
}

/// 【框选翻译】【服务启动】用 state 当前会话原文和目标启动任务，overlay 提供完成通知，返回配置或启动错误
fn translate(state: &mut EditorState, overlay: &mut dyn ScreenOverlay) -> AppResult {
    let session = state
        .selection_translation
        .session
        .as_ref()
        .or(state.selection_translation.draft.as_ref())
        .ok_or("No text to translate")?;
    let target = state
        .selection_translation
        .pending
        .as_ref()
        .ok_or("No translation target")?;
    let mut config = crate::config::translation::current()?;
    config.target_language = target.clone();
    let wake = overlay.task_wake()?;
    let request = Request::image(
        &session.source,
        (
            session.region.original.width(),
            session.region.original.height(),
        ),
    );
    state.selection_translation.task = Some(Task::start_request(
        &config,
        request,
        target,
        &crate::config::translation::path()?,
        move || wake.notify(),
    )?);
    Ok(())
}

/// 【框选翻译】【失败恢复】清理 state 未完成身份并显示 error 对应提示，dirty 返回恢复范围，已有翻译保持显示
fn failed(state: &mut EditorState, error: Box<dyn std::error::Error>, dirty: &mut u32) {
    log::error!("【框选翻译】【任务失败】{error}");
    state.selection_translation.pending = None;
    state.selection_translation.draft = None;
    state.selection_translation.recognizing = false;
    state.selection_translation.ocr_deadline = None;
    feedback(
        state,
        if error.to_string() == "No text to translate" {
            "No text to translate"
        } else {
            "Translation failed"
        },
        dirty,
    );
    super::super::panels::toolbar::update_toolbar(state, dirty);
}

/// 【框选翻译】【完成处理】轮询 state 实际运行和回收任务，overlay 接入后续服务，dirty 返回新显示，不接受已取消身份
pub(in crate::app) fn poll(
    state: &mut EditorState,
    overlay: &mut dyn ScreenOverlay,
    dirty: &mut u32,
) {
    state
        .selection_translation
        .retired
        .retain(|task| !task.is_finished());
    state
        .selection_translation
        .retired_ocr
        .retain(|task| !task.is_finished());
    if state
        .selection_translation
        .ocr_deadline
        .is_some_and(|deadline| Instant::now() >= deadline)
    {
        cancel(state, dirty);
        feedback(state, "OCR timed out", dirty);
    }
    let result = state
        .selection_translation
        .ocr
        .as_mut()
        .and_then(super::recognition::Task::take_result);
    if let Some(result) = result
        && state.selection_translation.recognizing
    {
        let task = state.selection_translation.ocr.take().unwrap();
        if !task.is_finished() {
            state.selection_translation.retired_ocr.push(task);
        }
        state.selection_translation.recognizing = false;
        state.selection_translation.ocr_deadline = None;
        match result {
            Ok(text)
                if state
                    .selection_translation
                    .draft
                    .as_mut()
                    .is_some_and(|s| s.recognized(&text)) =>
            {
                if let Err(error) = translate(state, overlay) {
                    failed(state, error, dirty);
                }
            }
            Ok(_) => failed(state, "No text to translate".into(), dirty),
            Err(error) => failed(state, error.into(), dirty),
        }
    }
    let result = state
        .selection_translation
        .task
        .as_mut()
        .and_then(Task::take_result);
    let Some(result) = result else { return };
    let task = state.selection_translation.task.take().unwrap();
    if !task.is_finished() {
        state.selection_translation.retired.push(task);
    }
    let Some(target) = state.selection_translation.pending.take() else {
        return;
    };
    match result {
        Ok(output) => {
            let session = state
                .selection_translation
                .session
                .as_mut()
                .or(state.selection_translation.draft.as_mut())
                .unwrap();
            let mut texts = vec![String::new(); session.source.len()];
            for segment in output.segments {
                if let Ok(index) = usize::try_from(segment.id)
                    && let Some(text) = texts.get_mut(index)
                {
                    *text = segment.text;
                }
            }
            let blocks = layout::translated(
                &session.region.original,
                &session.source,
                &texts,
                &mut state.text.font_system,
            );
            let pixels = paint::bake(
                &session.region.original,
                &blocks,
                &mut state.text.font_system,
                &mut state.text.swash_cache,
            );
            if session.complete(target, pixels, blocks) {
                if state.selection_translation.session.is_none() {
                    state.selection_translation.session = state.selection_translation.draft.take();
                }
                apply(state, dirty);
            } else {
                failed(state, "No text to translate".into(), dirty);
            }
        }
        Err(error) => failed(state, error.into(), dirty),
    }
    super::super::panels::toolbar::update_toolbar(state, dirty);
}

/// 【框选翻译】【请求取消】解除 state 当前身份并异步回收工作，dirty 返回工具栏范围，不改变已完成缓存
pub(in crate::app) fn cancel(state: &mut EditorState, dirty: &mut u32) {
    state.selection_translation.requested = None;
    state.selection_translation.pending = None;
    state.selection_translation.draft = None;
    state.selection_translation.recognizing = false;
    state.selection_translation.ocr_deadline = None;
    if let Some(mut task) = state.selection_translation.ocr.take() {
        task.cancel();
        state.selection_translation.retired_ocr.push(task);
    }
    if let Some(mut task) = state.selection_translation.task.take() {
        task.cancel();
        state.selection_translation.retired.push(task);
    }
    super::super::panels::toolbar::update_toolbar(state, dirty);
}
