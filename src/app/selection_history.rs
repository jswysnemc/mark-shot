//! 【历史选区】【编辑器接入】截图完成选框后记录，逗号与句号恢复前后选区，文件编辑不参与

use crate::{
    editor::EditorState,
    keys::{Chord, Key},
};

/// 【历史选区】【会话加载】按 state 读取当前配置目录下的历史，图片编辑和路径不可用时保持空历史
pub(super) fn load(state: &mut EditorState) {
    if state.file_canvas.is_none()
        && let Ok(path) = crate::config::preferences::path()
        && let Some(parent) = path.parent()
    {
        state.selection_history = crate::features::selection_history::History::load(
            parent.join("selection-history.json"),
        );
    }
}

/// 【历史选区】【选区提交】按 state 保存已完成的有效截图选区，不记录文件编辑、拖动过程或不足八像素的区域
pub(crate) fn record(state: &mut EditorState) {
    if state.file_canvas.is_some() || state.input.mouse_down || state.tool_active {
        return;
    }
    if let Some(rect) = state.selection.zone
        && rect.width() >= 8.0
        && rect.height() >= 8.0
    {
        state.selection_history.record(rect, &state.placements);
    }
}

/// 【历史选区】【快捷切换】按 chord 恢复 state 的前后选区，dirty 返回全窗口刷新；输入字段和拖动由调用方优先处理
pub(super) fn key(state: &mut EditorState, chord: Option<Chord>, dirty: &mut u32) -> bool {
    let Some(chord) = chord else { return false };
    if chord.mods != Default::default()
        || !matches!(chord.key, Key::Char(',' | '.'))
        || state.file_canvas.is_some()
        || state.input.mouse_down
        || state.tool_active
        || state.ann_drag.is_some()
        || state.pending.is_some()
        || state.text_session.is_some()
        || state.upload.busy()
    {
        return false;
    }
    if let Some(rect) = state
        .selection_history
        .step(chord.key == Key::Char('.'), &state.placements)
    {
        state.selection.zone = Some(rect);
        state.selection.creating = false;
        state
            .selection
            .set_drag(crate::types::SelectionHandle::None, None, None);
        state.input.clicks.clear();
        state.fullscreen_scope = false;
        super::input::refresh_panels(state, dirty);
        crate::editor::dirty::mark_all_dirty(dirty, state.placements.len());
        for monitor in 0..state.placements.len() {
            if let Some(bounds) = state.placements[monitor].rect() {
                state.damage_local(monitor, bounds);
            }
        }
    }
    true
}
