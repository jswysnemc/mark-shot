//! 【文字光标】【会话调度】将编辑焦点与相位映射到最小动态损坏，不更新持久标注图层

use std::time::Instant;

use cosmic_text::Edit;
use tiny_skia::Rect;

use crate::{
    editor::{DamageZone, EditorState, dirty::apply_damage_rects},
    types::AnnotationShape,
    ui::panel::UiPanel,
};

/// 【文字光标】【编辑焦点】按 state 的表面焦点、字段和菜单返回正文是否获得输入；字符选区不禁用原版光标
fn focused(state: &EditorState) -> bool {
    state.input.keyboard_focus.is_some()
        && !state.ui_hidden
        && state.text_context_menu.is_none()
        && !state.font_panel.open
        && !state.color_popover.is_visible()
        && !state.settings_panel.is_editing()
        && state.selected_tool != crate::tools::Tool::Ocr
        && state.text_session.as_ref().is_none_or(|session| {
            session
                .composition
                .as_ref()
                .is_none_or(|composition| composition.preedit.cursor.is_some())
        })
}

/// 【文字光标】【逻辑区域】从 state 真实布局返回当前光标全局范围，保留两像素容差覆盖分数缩放取整，不重排正文
fn rect(state: &EditorState) -> Option<Rect> {
    let session = state.text_session.as_ref()?;
    let AnnotationShape::Text {
        start, appearance, ..
    } = &session.working.shape
    else {
        return None;
    };
    let padding = crate::features::text::geometry::padding(appearance.size_scale);
    let editor = state.text.editors.get(&session.presented().id)?;
    let cursor = editor.cursor();
    editor.with_buffer(|buffer| {
        buffer.layout_runs().find_map(|run| {
            let x = run.cursor_position(&cursor)?;
            Rect::from_xywh(
                start.0 + padding.0 + x - 2.0,
                start.1 + padding.1 + run.line_top - 2.0,
                5.0,
                run.line_height + 4.0,
            )
        })
    })
}

/// 【文字光标】【局部损坏】将 state 当前光标范围交给 dirty，只更新动态画布，不使缓存标注图层失效，无返回值
fn damage(state: &mut EditorState, dirty: &mut u32) {
    if let Some(rect) = rect(state) {
        state.damage_rects.push(DamageZone::Global(rect));
        apply_damage_rects(state, dirty);
    }
}

/// 【文字光标】【焦点与截止】用 now 同步 state 焦点并处理一次到期切换，dirty 返回局部区域，无返回值
pub(crate) fn tick(state: &mut EditorState, now: Instant, dirty: &mut u32) {
    let active = focused(state);
    let Some(session) = state.text_session.as_mut() else {
        return;
    };
    let focus_changed = session.caret.focus(active, now);
    let phase_changed = session.caret.tick(now);
    if focus_changed || phase_changed {
        damage(state, dirty);
    }
}

/// 【文字光标】【事件活动】按 state 恢复正文光标可见相位，不推迟已有截止；dirty 返回需要恢复的动态区域，无返回值
pub(crate) fn activity(state: &mut EditorState, dirty: &mut u32) {
    if state
        .text_session
        .as_mut()
        .is_some_and(|session| session.caret.activity())
    {
        damage(state, dirty);
    }
}

/// 【文字光标】【正文鼠标】用 now 重建 state 正文光标周期，dirty 返回需要更新的可见相位，不影响选区或历史，无返回值
pub(crate) fn pointer_activity(state: &mut EditorState, now: Instant, dirty: &mut u32) {
    if state
        .text_session
        .as_mut()
        .is_some_and(|session| session.caret.pointer_activity(now))
    {
        damage(state, dirty);
    }
}

/// 【文字光标】【绘制相位】返回 state 当前正文光标是否可见，没有事务时返回假，不修改字符选区
pub(crate) fn visible(state: &EditorState) -> bool {
    state.text_session.as_ref().is_some_and(|session| {
        session.caret.visible()
            && session
                .composition
                .as_ref()
                .is_none_or(|composition| composition.preedit.cursor.is_some())
    })
}
