//! 【选区尺寸】【面板布局】按实际选区和可见障碍物重新定位，绘制与命中共同保留工具栏层级

use crate::{
    editor::EditorState,
    features::selection_size::placement,
    renderer::Painter,
    types::Mode,
    ui::{
        language::Language,
        panel::UiPanel,
        selection_size::{
            cache::{self, Overlay},
            state::Panel,
        },
    },
};
use tiny_skia::Rect;

/// 【选区尺寸】【面板损坏】为 state 当前面板登记局部动态区域，dirty 返回所属显示器，无返回值
pub(in crate::app) fn damage(state: &mut EditorState, dirty: &mut u32) {
    let area = state.selection_size.panel.as_mut().and_then(|panel| {
        panel.dirty = true;
        panel.rect.map(|rect| (panel.monitor, rect))
    });
    if let Some((monitor, rect)) = area {
        state.damage_local(monitor, rect);
        crate::editor::dirty::mark_dirty(dirty, monitor);
    }
}

/// 【选区尺寸】【必须避让】按 state 和 monitor 返回文字编辑框与形状弹层，空障碍不占用位置
fn hard_obstacles(state: &EditorState, monitor: usize) -> Vec<Rect> {
    let placement = &state.placements[monitor];
    state
        .text_session
        .as_ref()
        .and_then(|session| {
            state
                .canvas_local_rect(session.presented().bbox, monitor)
                .and_then(|rect| rect.intersect(&placement.rect()?))
        })
        .into_iter()
        .chain(
            state
                .marker_popover
                .rect()
                .filter(|_| state.marker_popover.monitor_idx == monitor),
        )
        .collect()
}

/// 【选区尺寸】【面板更新】按 state 实际像素和工具栏刷新位置，dirty 返回旧新范围；用户待提交文字保持不变，无返回值
pub(in crate::app) fn update(state: &mut EditorState, dirty: &mut u32) {
    let old = state
        .selection_size
        .panel
        .as_ref()
        .and_then(|panel| panel.rect.map(|rect| (panel.monitor, rect)));
    let creating = state.selection.creating;
    let dimensions = super::apply::dimensions(state);
    let wanted = state.mode == Mode::Editor
        && !state.ui_hidden
        && !state.fullscreen_scope
        && !creating
        && super::apply::usable(state);
    if !wanted {
        if let Some(panel) = &mut state.selection_size.panel {
            panel.hide();
        }
        super::popup::close(state, dirty);
        state.selection_size.field_drag = None;
    } else {
        let (values, maximum) = dimensions.unwrap();
        let language = Language::current();
        let monitor = state
            .selection_size
            .panel
            .as_ref()
            .filter(|panel| panel.editing().is_some() || state.selection_size.popup.is_some())
            .map_or(state.input.pointer.monitor_idx, |panel| panel.monitor);
        let selection = state
            .canvas_local_rect(state.selection.zone.unwrap(), monitor)
            .and_then(|rect| rect.intersect(&state.placements[monitor].rect()?));
        let hard = hard_obstacles(state, monitor);
        let all = hard
            .iter()
            .copied()
            .chain(
                [&state.toolbar, &state.action_toolbar]
                    .into_iter()
                    .filter(|toolbar| toolbar.monitor_idx == monitor)
                    .filter_map(|toolbar| toolbar.rect()),
            )
            .chain(
                state
                    .settings_panel
                    .rect()
                    .filter(|_| state.settings_panel.monitor_idx == monitor),
            )
            .collect::<Vec<_>>();
        let panel = state
            .selection_size
            .panel
            .get_or_insert_with(|| Panel::new(language));
        let changed = panel.sync(values, maximum, state.selection.aspect_ratio, language);
        let bounds = state.placements[monitor].rect().unwrap();
        let preferred = selection
            .and_then(|selection| placement::hud(selection, panel.layout.size, bounds, &all));
        let fallback = preferred.or_else(|| {
            selection
                .and_then(|selection| placement::hud(selection, panel.layout.size, bounds, &hard))
        });
        let under = preferred.is_none() && fallback.is_some();
        let moved =
            panel.monitor != monitor || panel.rect != fallback || panel.under_toolbars != under;
        panel.monitor = monitor;
        panel.rect = fallback;
        panel.under_toolbars = under;
        panel.dirty |= moved;
        if fallback.is_none() {
            panel.hide();
            super::popup::close(state, dirty);
        } else if changed || moved {
            damage(state, dirty);
        }
    }
    let new = state
        .selection_size
        .panel
        .as_ref()
        .and_then(|panel| panel.rect.map(|rect| (panel.monitor, rect)));
    if old != new {
        for (monitor, rect) in old.into_iter().chain(new) {
            state.damage_local(monitor, rect);
            crate::editor::dirty::mark_dirty(dirty, monitor);
        }
    }
    super::caret::tick(state, std::time::Instant::now(), dirty);
}

/// 【选区尺寸】【绘制准备】按 state 当前设备比例准备可见面板缓存，不在静止帧重新排版，无参数以外返回值
pub(in crate::app) fn prepare(state: &mut EditorState) {
    if let Some(panel) = state
        .selection_size
        .panel
        .as_mut()
        .filter(|panel| panel.rect.is_some())
    {
        cache::panel(
            panel,
            state.placements[panel.monitor].scale,
            state.selection_size.popup.is_some(),
            state.selection_size.caret.visible(),
        );
    }
}

/// 【选区尺寸】【尺寸叠层】按 state 和 monitor 返回当前界面缓存，只用于屏幕合成，不属于图片导出
pub(in crate::app) fn overlay(state: &super::State, monitor: usize) -> Option<Overlay<'_>> {
    let panel = state
        .panel
        .as_ref()
        .filter(|panel| panel.monitor == monitor)?;
    Some(Overlay {
        rect: panel.rect?,
        pixels: panel.cache.as_ref()?,
        under_toolbars: panel.under_toolbars,
    })
}

/// 【选区尺寸】【回退弹层绘制】按 state 在 monitor 合成非原生比例列表，dirty 限制动态范围，无返回值
pub(in crate::app) fn draw_popup(state: &mut EditorState, monitor: usize, dirty: Option<Rect>) {
    if let Some(popup) = state
        .selection_size
        .popup
        .as_ref()
        .filter(|popup| popup.monitor == monitor && !popup.native)
        && let Some(pixels) = &popup.cache
    {
        cache::composite(
            &mut Painter::new(&mut state.canvas[monitor])
                .with_scale(state.placements[monitor].scale),
            &Overlay {
                rect: popup.rect,
                pixels,
                under_toolbars: false,
            },
            dirty,
        );
    }
}
