//! 【选区尺寸】【原生弹层】比例列表使用现有 Wayland 抓取与身份隔离，关闭后同步释放角色

use crate::{
    AppResult,
    backend::ScreenOverlay,
    editor::EditorState,
    features::selection_size::placement,
    types::popup::{PopupEvent, PopupFrame},
    ui::selection_size::{
        cache,
        layout::POPUP_SIZE,
        ratio_popup::{Event, Popup},
    },
};
use tiny_skia::Rect;

/// 【选区尺寸】【弹层关闭】关闭 state 当前列表并登记旧范围，dirty 返回动态损坏，无返回值
pub(in crate::app) fn close(state: &mut EditorState, dirty: &mut u32) {
    if let Some(popup) = state.selection_size.popup.take() {
        state.damage_local(popup.monitor, popup.rect);
        crate::editor::dirty::mark_dirty(dirty, popup.monitor);
        super::panels::damage(state, dirty);
    }
}

/// 【选区尺寸】【比例列表打开】按 state 当前比例触发区打开列表，dirty 返回关闭字段与显示范围，无返回值
pub(in crate::app) fn open(state: &mut EditorState, dirty: &mut u32) {
    if state.selection_size.popup.is_some() {
        close(state, dirty);
        return;
    }
    let Some(panel) = &mut state.selection_size.panel else {
        return;
    };
    let Some(bounds) = panel.rect else { return };
    for field in &mut panel.fields {
        field.cancel();
    }
    let monitor = panel.monitor;
    let anchor = Rect::from_xywh(
        bounds.left() + panel.layout.ratio.left(),
        bounds.top() + panel.layout.ratio.top(),
        panel.layout.ratio.width(),
        panel.layout.ratio.height(),
    )
    .unwrap();
    let rect = placement::popup(
        anchor,
        POPUP_SIZE,
        state.placements[monitor].rect().unwrap(),
    );
    let id = state.next_menu_id;
    state.next_menu_id = state.next_menu_id.wrapping_add(1).max(1);
    state.selection_size.popup = Some(Popup::new(id, monitor, rect, state.selection.aspect_ratio));
    state.damage_local(monitor, rect);
    crate::editor::dirty::mark_dirty(dirty, monitor);
    super::panels::damage(state, dirty);
    super::caret::tick(state, std::time::Instant::now(), dirty);
}

/// 【选区尺寸】【列表动作】按 event 在 state 关闭列表或应用比例，dirty 返回实际选区与界面变化，无返回值
pub(in crate::app) fn chosen(state: &mut EditorState, event: Event, dirty: &mut u32) {
    match event {
        Event::None => {}
        Event::Close => close(state, dirty),
        Event::Choose(value) => {
            close(state, dirty);
            super::apply::ratio(state, value, dirty);
        }
    }
}

/// 【选区尺寸】【列表事件】按 id 将 event 交给 state 当前列表，dirty 返回变化，过期身份返回未消费
pub(in crate::app) fn event(
    state: &mut EditorState,
    id: u64,
    event: PopupEvent,
    dirty: &mut u32,
) -> bool {
    if state
        .selection_size
        .popup
        .as_ref()
        .is_none_or(|popup| popup.id != id)
    {
        return false;
    }
    if let PopupEvent::Key {
        chord: Some(chord), ..
    } = &event
        && crate::theme::is_toggle(*chord)
    {
        super::super::theme_switch::toggle(state, dirty);
        return true;
    }
    let popup = state.selection_size.popup.as_mut().unwrap();
    let action = match event {
        PopupEvent::Configured { position } => {
            popup.rect = Rect::from_xywh(
                position.0 as f32,
                position.1 as f32,
                popup.rect.width(),
                popup.rect.height(),
            )
            .unwrap();
            Event::None
        }
        PopupEvent::Pointer { x, y } => {
            popup.pointer((x, y));
            Event::None
        }
        PopupEvent::Button {
            x,
            y,
            button,
            pressed,
        } => popup.button((x, y), button, pressed),
        PopupEvent::Key { chord, .. } => {
            chord.map_or(Event::None, |chord| popup.key(chord.key, chord.mods.shift))
        }
        PopupEvent::Closed => Event::Close,
        _ => Event::None,
    };
    let damage = (!popup.native && popup.dirty).then_some((popup.monitor, popup.rect));
    if let Some((monitor, rect)) = damage {
        state.damage_local(monitor, rect);
        crate::editor::dirty::mark_dirty(dirty, monitor);
    }
    chosen(state, action, dirty);
    true
}

/// 【选区尺寸】【列表协议同步】按 state 当前缓冲交给 overlay，返回是否拥有弹层角色；静止列表不重复提交
pub(in crate::app) fn sync(
    state: &mut EditorState,
    overlay: &mut dyn ScreenOverlay,
) -> AppResult<bool> {
    let Some(popup) = &mut state.selection_size.popup else {
        return Ok(false);
    };
    if popup.native && !popup.dirty {
        return Ok(true);
    }
    let scale = state.placements[popup.monitor].scale;
    let scale = if overlay.hidpi() {
        scale
    } else {
        scale.max(1.0).ceil()
    };
    if !cache::popup(
        popup,
        state.selection_size.panel.as_mut().unwrap(),
        scale,
        crate::ui::language::Language::current(),
    ) {
        return Err("selection ratio popup allocation failed".into());
    }
    let pixels = popup.cache.as_ref().unwrap();
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
