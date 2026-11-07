//! 【框选翻译】【面板衔接】语言列表使用原生抓取，结果栏按当前选区和障碍物重新定位

use crate::{
    AppResult,
    backend::ScreenOverlay,
    editor::EditorState,
    features::selection_translation::placement,
    renderer::Painter,
    types::popup::PopupFrame,
    ui::{
        panel::UiPanel,
        selection_translation::{draw, language_popup::Popup, result_bar::Bar},
        toolbar::{Integration, ToolbarButton, ToolbarItem},
    },
};
use tiny_skia::{Pixmap, Rect};

/// 【框选翻译】【弹层关闭】关闭 state 语言列表，dirty 返回旧范围，后续协议同步先销毁角色
pub(in crate::app) fn close(state: &mut EditorState, dirty: &mut u32) {
    if let Some(popup) = state.selection_translation.popup.take() {
        state.damage_local(popup.monitor, popup.rect);
        crate::editor::dirty::mark_dirty(dirty, popup.monitor);
    }
    super::super::panels::toolbar::update_toolbar(state, dirty);
}

/// 【框选翻译】【按钮锚点】从 state 的可见工具栏返回翻译按钮及方向，未显示时为空
fn anchor(state: &EditorState) -> Option<(usize, Rect, bool)> {
    for toolbar in [&state.action_toolbar, &state.toolbar] {
        let Some(rect) = toolbar.rect() else { continue };
        if let Some(index) = toolbar.items.iter().position(|item| {
            *item == ToolbarItem::Button(ToolbarButton::Integration(Integration::Translate))
        }) && let Some(cell) = toolbar.cells.get(index)
        {
            return Some((
                toolbar.monitor_idx,
                Rect::from_xywh(
                    rect.left() + cell.left(),
                    rect.top() + cell.top(),
                    cell.width(),
                    cell.height(),
                )
                .unwrap(),
                toolbar.size.1 > toolbar.size.0,
            ));
        }
    }
    None
}

/// 【框选翻译】【语言弹层】在 state 从工具按钮或浮条打开语言列表，dirty 返回关闭其他面板和显示范围
pub(in crate::app) fn open(state: &mut EditorState, from_bar: bool, dirty: &mut u32) {
    if state.selection_translation.busy()
        || state.tool_active
        || state.input.mouse_down
        || state.ann_drag.is_some()
        || state.selection.zone.is_none()
    {
        return;
    }
    if state.selection_translation.popup.is_some() {
        close(state, dirty);
        return;
    }
    crate::editor::text_session::commit(state, dirty);
    if state.pending.is_some() {
        return;
    }
    super::super::external_actions::close(state, dirty);
    super::super::panels::color::close_color_popover(state, dirty);
    super::super::panels::text::font_popup::close(state, dirty);
    super::super::panels::model::close_model_popover(state, dirty);
    super::super::panels::marker::close(state, dirty);
    super::super::panels::ruler::close(state, dirty);
    super::super::panels::number::close(state, dirty);
    super::super::panels::line::close(state, dirty);
    super::super::panels::filter::close(state, dirty);
    state.text_context_menu = None;
    let target = state
        .selection_translation
        .session
        .as_ref()
        .map(|s| s.language.clone())
        .unwrap_or_else(|| {
            crate::config::translation::current()
                .unwrap_or_default()
                .target_language
        });
    let fallback = state.input.pointer.monitor_idx;
    let anchor = if from_bar {
        state
            .selection_translation
            .bar
            .as_ref()
            .map(|bar| (bar.monitor, bar.rect, false))
    } else {
        None
    }
    .or_else(|| anchor(state));
    let (monitor, button, vertical) = anchor.unwrap_or((
        fallback,
        Rect::from_xywh(
            state.input.pointer.local.0 as f32,
            state.input.pointer.local.1 as f32,
            1.0,
            1.0,
        )
        .unwrap(),
        false,
    ));
    let id = state.next_menu_id;
    state.next_menu_id = state.next_menu_id.wrapping_add(1).max(1);
    let fonts = state
        .selection_translation
        .ui_fonts
        .get_or_insert_with(crate::fonts::ocr_ui::new);
    let mut popup = Popup::new(id, monitor, &target, fonts);
    popup.rect = placement::language(
        button,
        vertical,
        (popup.rect.width(), popup.rect.height()),
        state.placements[monitor].rect().unwrap(),
    );
    state.damage_local(monitor, popup.rect);
    crate::editor::dirty::mark_dirty(dirty, monitor);
    state.selection_translation.popup = Some(popup);
    super::super::panels::toolbar::update_toolbar(state, dirty);
}

/// 【框选翻译】【浮条刷新】按 state 当前区域交集及工具栏刷新浮条，dirty 返回旧新位置，缓存语言不受选区移动影响
pub(in crate::app) fn update(state: &mut EditorState, dirty: &mut u32) {
    let old_look = state.selection_translation.bar.as_ref().map(|bar| {
        (
            bar.monitor,
            bar.rect,
            bar.label.clone(),
            bar.translated,
            bar.hovered,
        )
    });
    let old = state
        .selection_translation
        .bar
        .as_ref()
        .map(|bar| (bar.monitor, bar.rect));
    let visible = !state.ui_hidden
        && !state.tool_active
        && state
            .selection_translation
            .session
            .as_ref()
            .is_some_and(|s| {
                state
                    .selection
                    .zone
                    .is_some_and(|r| r.intersect(&s.region.bounds).is_some())
            });
    if !visible {
        state.selection_translation.bar = None;
    } else {
        let session = state.selection_translation.session.as_ref().unwrap();
        let label = crate::translation::language::display_for(
            &session.language,
            crate::ui::language::Language::current(),
        )
        .to_owned();
        let fonts = state
            .selection_translation
            .ui_fonts
            .get_or_insert_with(crate::fonts::ocr_ui::new);
        let mut bar = Bar::new(
            label,
            session.showing_translation && session.current().is_some(),
            fonts,
        );
        let monitor = state.input.pointer.monitor_idx;
        let placement = &state.placements[monitor];
        let region = state
            .canvas_local_rect(session.region.bounds, monitor)
            .and_then(|rect| rect.intersect(&placement.rect()?));
        if let Some(region) = region {
            let obstacles = [&state.toolbar, &state.action_toolbar]
                .iter()
                .filter(|p| p.monitor_idx == monitor)
                .filter_map(|p| p.rect())
                .chain(
                    state
                        .settings_panel
                        .rect()
                        .filter(|_| state.settings_panel.monitor_idx == monitor),
                )
                .chain(
                    state
                        .selection_size
                        .panel
                        .as_ref()
                        .filter(|panel| panel.monitor == monitor)
                        .and_then(|panel| panel.rect),
                )
                .collect::<Vec<_>>();
            bar.rect = placement::result(
                region,
                (bar.rect.width(), bar.rect.height()),
                placement.rect().unwrap(),
                &obstacles,
            );
            bar.monitor = monitor;
            if let Some(previous) = &state.selection_translation.bar {
                bar.pressed = previous.pressed;
            }
            bar.hovered = bar.hit(state.input.pointer.local);
            state.selection_translation.bar = Some(bar);
        } else {
            state.selection_translation.bar = None;
        }
    }
    let new = state
        .selection_translation
        .bar
        .as_ref()
        .map(|bar| (bar.monitor, bar.rect));
    let new_look = state.selection_translation.bar.as_ref().map(|bar| {
        (
            bar.monitor,
            bar.rect,
            bar.label.clone(),
            bar.translated,
            bar.hovered,
        )
    });
    if old_look == new_look {
        return;
    }
    for (monitor, rect) in old.into_iter().chain(new) {
        state.damage_local(monitor, rect);
        crate::editor::dirty::mark_dirty(dirty, monitor);
    }
}

/// 【框选翻译】【协议同步】先按 state 更新冻结背景和原生语言列表，overlay 接收有界缓冲，返回是否拥有当前弹层
pub(in crate::app) fn sync(
    state: &mut EditorState,
    overlay: &mut dyn ScreenOverlay,
) -> AppResult<bool> {
    if std::mem::take(&mut state.selection_translation.background_dirty) {
        for (index, capture) in state.captures.iter().enumerate() {
            overlay.set_background(
                index,
                capture.pixmap.data(),
                capture.pixmap.width(),
                capture.pixmap.height(),
            )?;
        }
    }
    let Some(popup) = &mut state.selection_translation.popup else {
        return Ok(false);
    };
    if popup.native && !popup.dirty && popup.theme == Some(crate::theme::Mode::current()) {
        return Ok(true);
    }
    let scale = state.placements[popup.monitor].scale;
    let scale = if overlay.hidpi() {
        scale
    } else {
        scale.max(1.0).ceil()
    };
    if popup.dirty || popup.theme != Some(crate::theme::Mode::current()) || popup.pixels.is_none() {
        let mut pixels = Pixmap::new(
            (popup.rect.width() * scale).ceil() as u32,
            (popup.rect.height() * scale).ceil() as u32,
        )
        .ok_or("translation popup allocation failed")?;
        draw::popup(
            &mut Painter::new(&mut pixels)
                .with_scale(scale)
                .with_icons(&state.icons_cache)
                .with_text(
                    state.selection_translation.ui_fonts.as_mut().unwrap(),
                    state
                        .selection_translation
                        .ui_cache
                        .get_or_insert_with(cosmic_text::SwashCache::new),
                ),
            popup,
        );
        popup.pixels = Some(pixels);
        popup.dirty = false;
        popup.theme = Some(crate::theme::Mode::current());
    }
    let pixels = popup.pixels.as_ref().unwrap();
    popup.native = overlay.set_popup(Some(PopupFrame {
        id: popup.id,
        monitor_idx: popup.monitor,
        serial: state.input.popup_serial,
        anchor: (popup.rect.left(), popup.rect.top()),
        size: (popup.rect.width() as u32, popup.rect.height() as u32),
        scale,
        pixels: pixels.data(),
        pixel_size: (pixels.width(), pixels.height()),
    }))?;
    Ok(true)
}

/// 【框选翻译】【画布组件】在 state 当前 monitor 绘制浮条与非原生列表，dirty 限制合成区域，不进入导出图层
pub(in crate::app) fn draw(state: &mut EditorState, monitor: usize, dirty: Option<Rect>) {
    let Some(fonts) = state.selection_translation.ui_fonts.as_mut() else {
        return;
    };
    let mut painter = Painter::new(&mut state.canvas[monitor])
        .with_scale(state.placements[monitor].scale)
        .with_icons(&state.icons_cache)
        .with_text(
            fonts,
            state
                .selection_translation
                .ui_cache
                .get_or_insert_with(cosmic_text::SwashCache::new),
        );
    if let Some(bar) = state
        .selection_translation
        .bar
        .as_ref()
        .filter(|bar| bar.monitor == monitor)
    {
        painter.clipped_scene(dirty, |p| draw::bar(p, bar));
    }
    if let Some(popup) = state
        .selection_translation
        .popup
        .as_ref()
        .filter(|p| p.monitor == monitor && !p.native)
        && let Some(pixels) = &popup.pixels
    {
        painter.clipped_scene(dirty, |p| {
            let origin = p.origin();
            let scale = p.scale();
            let transform = tiny_skia::Transform::from_scale(
                popup.rect.width() * scale / pixels.width() as f32,
                popup.rect.height() * scale / pixels.height() as f32,
            )
            .post_translate(
                (popup.rect.left() - origin.0) * scale,
                (popup.rect.top() - origin.1) * scale,
            );
            p.canvas().draw_pixmap(
                0,
                0,
                pixels.as_ref(),
                &tiny_skia::PixmapPaint::default(),
                transform,
                None,
            );
        });
    }
}
