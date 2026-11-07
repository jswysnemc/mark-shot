//! 【文字弹层】【界面协议衔接】菜单自有缓冲交给 Wayland，原生角色销毁请求先于文字动作

use crate::{
    AppResult,
    backend::ScreenOverlay,
    editor::EditorState,
    renderer::Painter,
    types::popup::{PopupEvent, PopupFrame},
    ui::context_menu::Event,
};

/// 【文字弹层】【帧同步】用 overlay 同步 state 菜单，dirty 返回动作重绘范围；关闭表面后才执行动作，返回协议结果
pub fn sync(
    state: &mut EditorState,
    overlay: &mut dyn ScreenOverlay,
    dirty: &mut u32,
) -> AppResult {
    if state.text_context_menu.is_none() {
        overlay.set_popup(None)?;
        if let Some(command) = state.pending_text_menu_action.take() {
            crate::tools::text::session_input::command(state, command, dirty);
            crate::app::input::refresh_panels(state, dirty);
        }
        return Ok(());
    }
    let menu = state.text_context_menu.as_mut().unwrap();
    if menu.native && !menu.dirty && menu.drawn_theme == Some(crate::theme::Mode::current()) {
        return Ok(());
    }
    let scale = state.placements[menu.monitor_idx].scale;
    let scale = if overlay.hidpi() {
        scale
    } else {
        scale.max(1.0).ceil()
    };
    // 1. 【文字弹层】【绘制上下文】单像素承载字体和比例上下文，实际像素仅写入菜单自有缓存
    let mut context = tiny_skia::Pixmap::new(1, 1).unwrap();
    crate::ui::context_menu::draw::prepare(
        &mut Painter::new(&mut context)
            .with_scale(scale)
            .with_text(&mut state.text.font_system, &mut state.text.swash_cache),
        menu,
    );
    let pixels = menu
        .pixmap
        .as_ref()
        .ok_or("text popup buffer allocation failed")?;
    menu.native = overlay.set_popup(Some(PopupFrame {
        id: menu.id,
        monitor_idx: menu.monitor_idx,
        serial: state.input.popup_serial,
        anchor: menu.anchor,
        size: (menu.layout.size.0 as u32, menu.layout.size.1 as u32),
        scale,
        pixels: pixels.data(),
        pixel_size: (pixels.width(), pixels.height()),
    }))?;
    Ok(())
}

/// 【文字弹层】【输入交付】处理 state 当前 id 对应的 event，dirty 返回重绘范围；过期弹层事件一律丢弃，无返回值
pub fn event(state: &mut EditorState, id: u64, event: PopupEvent, dirty: &mut u32) {
    let Some(menu) = state
        .text_context_menu
        .as_mut()
        .filter(|menu| menu.id == id)
    else {
        return;
    };
    let event = match event {
        PopupEvent::Configured { position } => {
            menu.position = (position.0 as f32, position.1 as f32);
            Event::None
        }
        PopupEvent::Pointer { x, y } => menu.pointer(
            menu.monitor_idx,
            (
                f64::from(menu.position.0) + x,
                f64::from(menu.position.1) + y,
            ),
        ),
        PopupEvent::Leave => menu.pointer(usize::MAX, (0.0, 0.0)),
        PopupEvent::Button {
            x,
            y,
            button,
            pressed,
        } => menu.button(
            menu.monitor_idx,
            (
                f64::from(menu.position.0) + x,
                f64::from(menu.position.1) + y,
            ),
            button,
            pressed,
        ),
        PopupEvent::Key { chord, .. } => {
            chord.map_or(Event::None, |chord| menu.key(chord.key, chord.mods.shift))
        }
        PopupEvent::Closed => Event::Close,
        PopupEvent::Scroll { .. } => Event::None,
    };
    crate::app::text_context_menu::event(state, event, dirty);
}
