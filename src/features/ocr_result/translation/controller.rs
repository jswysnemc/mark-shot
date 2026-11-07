//! 【OCR 翻译】【任务交付】窗口主动发起一次任务，完成按身份更新译文，取消不等待工作线程

use super::super::{
    control_input::FocusTarget,
    window::{ResultWindow, View},
};
use super::{Translation, style};
use crate::{
    backend::window::WindowAction,
    translation::{Error, ErrorKind, task::Task},
    ui::text_editor::TextEditor,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use unicode_segmentation::UnicodeSegmentation;

#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
pub(in crate::features::ocr_result) mod tests;

/// 【OCR 翻译】【变化检测】按 window 的确认正文版本更新过期提示及译文字素统计，无返回值，静止正文不反复分配完整字符串
pub(crate) fn refresh(window: &mut ResultWindow) {
    let current = [
        (&window.source),
        (&window.translation.language),
        (&window.translation.editor),
    ]
    .map(|editor| editor.text_version());
    if current[0..2] != window.translation.versions[0..2] {
        let source = window.source.text();
        let target = window.translation.target();
        if window.translation.state.refresh(&source, &target) {
            window.translation.layout_dirty = true;
        }
    }
    if current[2] != window.translation.versions[2] {
        window.translation.characters = window.translation.editor.text().graphemes(true).count();
        window.translation.layout_dirty = true;
    }
    window.translation.versions = current;
}

/// 【OCR 翻译】【语言替换】按 translation、fonts 和 target 替换语言输入并创建新身份，无返回值；不修改译文或原文编辑历史
fn set_target(translation: &mut Translation, fonts: &mut cosmic_text::FontSystem, target: &str) {
    let target = crate::translation::language::display_for(target, translation.ui_language);
    crate::fonts::ocr_ui::ensure_text(fonts, target);
    translation.language = TextEditor::new_with_style(target, fonts, style(true));
    translation.language.single_line(fonts);
    translation
        .language
        .set_byte_selection(fonts, target.len(), target.len());
    translation.language_offset = 0.0;
    translation.layout_dirty = true;
}

/// 【OCR 翻译】【目标保存】确认 window 的自由目标输入，返回是否成功；失败恢复已保存值，不把候选或空输入保存到配置
pub(crate) fn commit_target(window: &mut ResultWindow) -> bool {
    let target = window.translation.target();
    if target == window.translation.saved_target {
        return true;
    }
    let saved = if let Some(path) = &window.translation.config_path {
        crate::config::translation::save_target_at(path, &target)
    } else {
        crate::config::translation::save_target(&target)
    };
    match saved {
        Ok(target) => {
            window.translation.saved_target = target.clone();
            window.translation.config.target_language = target.clone();
            set_target(&mut window.translation, &mut window.fonts, &target);
            true
        }
        Err(error) => {
            log::error!("【OCR 翻译】【目标保存】保存失败: {error}");
            let target = window.translation.saved_target.clone();
            set_target(&mut window.translation, &mut window.fonts, &target);
            window.status = Some("Failed to save settings".into());
            false
        }
    }
}

/// 【OCR 翻译】【语言选择】按 index 选择原版稳定选项并确认 window 配置，返回重绘；忙碌时不接受选择
pub(crate) fn choose(window: &mut ResultWindow, index: usize) -> WindowAction {
    if window.translation.state.busy() {
        return WindowAction::None;
    }
    let Some(target) = crate::translation::language::OPTIONS.get(index) else {
        return WindowAction::None;
    };
    set_target(&mut window.translation, &mut window.fonts, target);
    commit_target(window);
    window.focus_target = FocusTarget::Language;
    window.translation.language.select_all(&mut window.fonts);
    refresh(window);
    WindowAction::Redraw
}

/// 【OCR 翻译】【开始任务】按 window 的原文和已确认目标启动任务，返回重绘；任务中不重复发送，配置失败留在译文区
pub(crate) fn start(window: &mut ResultWindow) -> WindowAction {
    if window.translation.state.busy() {
        return WindowAction::None;
    }
    commit_target(window);
    let source = window.source.text();
    let target = window.translation.saved_target.clone();
    if source.trim().is_empty() || target.trim().is_empty() {
        window.status = Some("No text to translate".into());
        return WindowAction::Redraw;
    }
    if window.view != View::Text {
        window.control(super::super::layout::Control::View(View::Text));
    }
    window.translation.state.visible = true;
    window.translation.layout_dirty = true;
    let config_path: Result<PathBuf, _> = window
        .translation
        .config_path
        .clone()
        .map(Ok)
        .unwrap_or_else(crate::config::translation::path);
    let result = config_path
        .map_err(|_| {
            Error::new(
                ErrorKind::Configuration,
                "translation config path is unavailable",
            )
        })
        .and_then(|path| {
            let config = crate::config::translation::read_at(&path)
                .or_else(|error| {
                    if error.kind() == std::io::ErrorKind::NotFound {
                        Ok(window.translation.config.clone())
                    } else {
                        Err(error)
                    }
                })
                .map_err(|_| {
                    Error::new(
                        ErrorKind::Configuration,
                        "translation config cannot be read",
                    )
                })?;
            let wake = window.translation.wake.clone().ok_or_else(|| {
                Error::new(
                    ErrorKind::StartFailed,
                    "translation window is not connected",
                )
            })?;
            Task::start(&config, &source, &target, &path, wake)
        });
    match result {
        Ok(task) => {
            window.translation.editor.cancel_preedit();
            window.translation.language.cancel_preedit();
            window.translation.state.begin(task.id, &source, &target);
            window.translation.task = Some(task);
            if window.focus_target == FocusTarget::Language {
                window.focus_control(FocusTarget::Button(
                    super::super::layout::Control::Translate,
                ));
            }
            let now = Instant::now();
            window.translation.animation_start = Some(now);
            window.translation.animation_phase = 0.0;
            window.translation.animation_deadline = Some(now + Duration::from_millis(50));
        }
        Err(error) => window.translation.state.fail(error),
    }
    refresh(window);
    WindowAction::Redraw
}

/// 【OCR 翻译】【取消任务】按 explicit 清理 window 所属任务并保留译文，无返回值，窗口线程不会连接未结束工作线程
pub(crate) fn cancel(window: &mut ResultWindow, explicit: bool) {
    let source = window.source.text();
    let target = window.translation.target();
    window.translation.state.cancel(&source, &target, explicit);
    retire(window);
    window.translation.animation_start = None;
    window.translation.animation_deadline = None;
    window.translation.layout_dirty = true;
}

/// 【OCR 翻译】【取消句柄保留】取消 window 当前任务并保存待回收句柄，无返回值；交互线程只通知，窗口退出时仍能等待所有所属进程清理
fn retire(window: &mut ResultWindow) {
    if let Some(mut task) = window.translation.task.take() {
        task.cancel();
        window.translation.retired.push(task);
        window.translation.retire_deadline = Some(Instant::now() + Duration::from_millis(5));
    }
}

/// 【OCR 翻译】【显示切换】切换 window 翻译区，返回重绘；空译文首次展开才请求，已有文本重开不重译
pub(crate) fn toggle(window: &mut ResultWindow) -> WindowAction {
    if window.translation.state.visible {
        cancel(window, false);
        window.translation.state.visible = false;
        if matches!(
            window.focus_target,
            FocusTarget::Translation | FocusTarget::Language
        ) {
            window.focus_control(FocusTarget::Content);
        }
        WindowAction::Redraw
    } else {
        window.translation.state.visible = true;
        window.translation.ratio = 0.6;
        window.translation.layout_dirty = true;
        if window.translation.editor.text().is_empty() {
            start(window)
        } else {
            WindowAction::Redraw
        }
    }
}

/// 【OCR 翻译】【后台唤醒】在 now 读取 window 的当前一次性结果，返回是否需要重绘，旧取消任务只有通知而没有可交付结果
pub(crate) fn wake(window: &mut ResultWindow, now: Instant) -> bool {
    if window
        .translation
        .retire_deadline
        .is_some_and(|deadline| deadline <= now)
    {
        window
            .translation
            .retired
            .retain(|task| !task.is_finished());
        window.translation.retire_deadline =
            (!window.translation.retired.is_empty()).then_some(now + Duration::from_millis(5));
    }
    let completed = window
        .translation
        .task
        .as_mut()
        .and_then(|task| task.take_result().map(|result| (task.id, result)));
    if let Some((id, result)) = completed {
        let source = window.source.text();
        let target = window.translation.target();
        if let Some(text) = window
            .translation
            .state
            .finish(id, result, &source, &target)
        {
            crate::fonts::ocr_ui::ensure_text(&mut window.fonts, &text);
            window.translation.editor =
                TextEditor::new_with_style(&text, &mut window.fonts, style(false));
        }
        retire(window);
        window.translation.animation_start = None;
        window.translation.animation_deadline = None;
        window.translation.layout_dirty = true;
        refresh(window);
        return true;
    }
    if window
        .translation
        .animation_deadline
        .is_some_and(|deadline| deadline <= now)
    {
        window.translation.animation_phase =
            window.translation.animation_start.map_or(0.0, |start| {
                (now.saturating_duration_since(start).as_millis() % 1500) as f32 / 1500.0
            });
        window.translation.animation_deadline = Some(now + Duration::from_millis(50));
        return window.translation.state.visible && window.view == View::Text;
    }
    false
}
