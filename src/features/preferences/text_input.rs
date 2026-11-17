//! 【设置窗口】【输入法桥接】统一路由搜索框、下拉搜索、行内文本编辑、翻译配置与翻译预览的输入法状态

use super::{
    catalog::{ENTRIES, Kind},
    layout::rect,
    translation::layout::{Hit as TranslationHit, Layout as TranslationLayout},
    window::Preferences,
};
use crate::{
    types::text_input::{ChangeCause, Edit, Focus, Snapshot},
    ui::text_editor::InputArea,
};
use tiny_skia::Rect;

/// 【设置窗口】【活动目标】返回 state 当前聚焦可编辑输入框的稳定身份，非文本编辑状态返回空值
pub(super) fn target(state: &Preferences) -> Option<u64> {
    if let Some(menu) = &state.dropdown {
        return menu.search.as_ref().map(|search| search.input_identity());
    }
    if state.search_focus {
        return Some(state.search.input_identity());
    }
    if state.translation_page() {
        if state.translation.preview.source.focused {
            return state
                .translation
                .preview
                .source
                .editor
                .as_ref()
                .map(|editor| editor.input_identity());
        }
        if let Some((field, editor)) = &state.translation.editing
            && (!field.secret() || state.translation.reveal)
        {
            return Some(editor.input_identity());
        }
        return None;
    }
    if let Some((index, field)) = &state.editing
        && matches!(ENTRIES[*index].kind, Kind::Text)
    {
        return Some(field.input_identity());
    }
    None
}

/// 【设置窗口】【候选矩形】将 area 约束在滚动视口 viewport 内，防止候选框漂移到窗口外侧
fn clamp_to_viewport(area: Rect, viewport: Rect) -> Rect {
    let top = area.top().clamp(
        viewport.top(),
        (viewport.bottom() - 4.0).max(viewport.top()),
    );
    rect(area.left(), top, area.width(), area.height())
}

/// 【设置窗口】【输入快照】按当前活动输入框计算坐标矩形并生成快照，静止无变更时返回空值
pub(super) fn snapshot(
    state: &mut Preferences,
    focus: Focus,
    target_id: u64,
    previous: Option<&Snapshot>,
    cause: ChangeCause,
) -> Option<Snapshot> {
    // 1. 【设置窗口】【下拉搜索】展开的可搜索下拉菜单优先持有输入法目标
    if state
        .dropdown
        .as_ref()
        .is_some_and(|menu| menu.search.is_some())
    {
        let bounds = state
            .dropdown
            .as_ref()
            .and_then(|menu| super::dropdown::layout::Layout::new(menu, state.size).search)?;
        let area = rect(
            bounds.left() + 29.0,
            bounds.top() + 3.0,
            bounds.width() - 37.0,
            bounds.height() - 6.0,
        );
        let menu = state.dropdown.as_mut()?;
        return menu.search.as_mut()?.input_snapshot(
            &mut state.fonts,
            area,
            focus,
            target_id,
            previous,
            cause,
        );
    }
    // 2. 【设置窗口】【顶栏搜索】顶部搜索框在获得焦点时提供精确输入区域
    if state.search_focus {
        let search = super::layout::search_rect(state);
        let area = rect(
            search.left() + 31.0,
            search.top() + 3.0,
            search.width() - 84.0,
            search.height() - 6.0,
        );
        return state.search.input_snapshot(
            &mut state.fonts,
            area,
            focus,
            target_id,
            previous,
            cause,
        );
    }
    // 3. 【设置窗口】【翻译页面】分别处理多行测试原文与单行服务参数输入框
    if state.translation_page() {
        let layout = TranslationLayout::new(state);
        if state.translation.preview.source.focused {
            let area = clamp_to_viewport(layout.preview.source_input(), layout.viewport);
            return state
                .translation
                .preview
                .source
                .editor
                .as_mut()?
                .input_snapshot(
                    &mut state.fonts,
                    InputArea {
                        rect: area,
                        offset: 0.0,
                    },
                    focus,
                    target_id,
                    previous,
                    cause,
                );
        }
        let (field, _) = state.translation.editing.as_ref()?;
        if field.secret() && !state.translation.reveal {
            return None;
        }
        let field = *field;
        let (_, bounds) = layout
            .controls
            .iter()
            .find(|(hit, _)| *hit == TranslationHit::Field(field))?;
        let area = clamp_to_viewport(
            rect(
                bounds.left() + 8.0,
                bounds.top() + 3.0,
                bounds.width() - 16.0,
                bounds.height() - 6.0,
            ),
            layout.viewport,
        );
        let (_, editor) = state.translation.editing.as_mut()?;
        return editor.input_snapshot(&mut state.fonts, area, focus, target_id, previous, cause);
    }
    // 4. 【设置窗口】【行内编辑】普通设置项文本与数值输入框复用表单布局范围
    let (index, _) = state.editing.as_ref()?;
    let index = *index;
    if !matches!(ENTRIES[index].kind, Kind::Text) {
        return None;
    }
    let layout = super::layout::Layout::new(state);
    let row = layout.rows.iter().find(|row| row.index == index)?;
    let area = clamp_to_viewport(super::layout::editor_rect(row), layout.viewport);
    let (_, editor) = state.editing.as_mut()?;
    editor.input_snapshot(&mut state.fonts, area, focus, target_id, previous, cause)
}

/// 【设置窗口】【候选清理】取消设置窗口内所有输入控件的未确认候选，返回是否有界面更新
pub(super) fn cancel(state: &mut Preferences) -> bool {
    let dropdown = state
        .dropdown
        .as_mut()
        .and_then(|menu| menu.search.as_mut())
        .is_some_and(|search| search.cancel_preedit());
    let search = state.search.cancel_preedit();
    let row = state
        .editing
        .as_mut()
        .is_some_and(|(_, field)| field.cancel_preedit());
    let translation_field = state
        .translation
        .editing
        .as_mut()
        .is_some_and(|(_, field)| field.cancel_preedit());
    let translation_source = state
        .translation
        .preview
        .source
        .editor
        .as_mut()
        .is_some_and(|editor| editor.cancel_preedit());
    dropdown || search || row || translation_field || translation_source
}

/// 【设置窗口】【文本提交】将输入法批次 edit 分发给顶栏搜索或当前行内输入框，返回是否消费
pub(super) fn apply(state: &mut Preferences, edit: Edit) -> bool {
    if state.search_focus {
        let update = state.search.input_method(&mut state.fonts, edit);
        if update.text_changed {
            state.scroll = 0;
        }
        return update.accepted;
    }
    if let Some((index, field)) = &mut state.editing
        && matches!(ENTRIES[*index].kind, Kind::Text)
    {
        return field.input_method(&mut state.fonts, edit).accepted;
    }
    false
}
