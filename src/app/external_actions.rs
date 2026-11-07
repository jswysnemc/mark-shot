//! 【外部动作】【编辑器交接】按需打开原版侧边列表，成功交付才关闭截图；失败保留冻结图像及标注历史

use crate::{
    AppResult,
    backend::ScreenOverlay,
    editor::{EditorState, dirty::apply_damage_rects},
    features::external_actions::{Action, desktop_apps, extensions, launch},
    renderer::{Painter, text::TextStyle},
    theme::Rgba,
    types::{MouseButton, ScrollEvent},
    ui::{
        external_panel::{ExternalPanel, Kind, layout::Layout, state::Event},
        panel::{UiPanel, emit_panel_damage},
        toolbar::{Integration, ToolbarButton, ToolbarItem},
    },
};

/// 【外部动作】【关闭面板】在 state 释放列表、图标及点击状态，dirty 返回原范围；不影响标注，无返回值
pub fn close(state: &mut EditorState, dirty: &mut u32) {
    if let Some(rect) = state.external_panel.rect() {
        emit_panel_damage(
            rect,
            state.external_panel.monitor_idx,
            &mut state.damage_rects,
            dirty,
        );
    }
    state.external_panel = ExternalPanel::default();
}

/// 【外部动作】【面板打开】在 state 切换 kind 并按原版工具栏定位，dirty 返回完整面板范围；入口前提交文字并关闭其他属性弹层
pub fn toggle(state: &mut EditorState, kind: Kind, dirty: &mut u32) {
    if state.external_panel.kind == Some(kind) {
        close(state, dirty);
        return;
    }
    if state.tool_active
        || state.input.mouse_down
        || state.pending.is_some()
        || state.ann_drag.is_some()
        || state.selection.zone.is_none()
    {
        return;
    }
    close(state, dirty);
    crate::editor::text_session::commit(state, dirty);
    super::panels::color::commit_color_field_edit(state, dirty);
    super::panels::settings::commit_stepper_text_edit(state, dirty);
    super::panels::color::close_color_popover(state, dirty);
    super::panels::model::close_model_popover(state, dirty);
    super::panels::text::font_popup::close(state, dirty);
    super::panels::marker::close(state, dirty);
    super::panels::ruler::close(state, dirty);
    super::panels::number::close(state, dirty);
    super::panels::line::close(state, dirty);
    super::panels::filter::close(state, dirty);
    state.input.clicks.clear();
    state.input.right_clicks.clear();
    let language = crate::ui::language::Language::current();
    let (actions, message) = match kind {
        Kind::OpenWith => {
            let actions = desktop_apps::discover(&desktop_apps::search_dirs())
                .into_iter()
                .map(Action::Desktop)
                .collect();
            (
                actions,
                language.text("No image desktop entries found").to_owned(),
            )
        }
        Kind::Extensions => match extensions::config_path() {
            Ok(path) => match extensions::load(&path) {
                Ok(commands) => (
                    commands.into_iter().map(Action::Extension).collect(),
                    language.format(
                        "No extension commands configured.\nCreate %1",
                        &[&path.to_string_lossy()],
                    ),
                ),
                Err(error) => (Vec::new(), error.to_string()),
            },
            Err(error) => (Vec::new(), error.to_string()),
        },
    };
    let toolbar = if state.fullscreen_scope {
        &state.toolbar
    } else {
        &state.action_toolbar
    };
    let monitor = toolbar.monitor_idx;
    let bounds = state.placements[monitor].size;
    let bounds = (bounds.0 as f32, bounds.1 as f32);
    let anchor = toolbar.rect().unwrap_or_else(|| fallback_rect(bounds));
    let mut pixels = tiny_skia::Pixmap::new(1, 1).unwrap();
    let mut painter = Painter::new(&mut pixels)
        .with_text(&mut state.text.font_system, &mut state.text.swash_cache);
    let width = actions
        .iter()
        .map(|action| painter.measure(action.name(), 16.0))
        .fold(0.0_f32, f32::max);
    let mut layout = Layout::new(kind, actions.len(), width, 36.0, bounds);
    if actions.is_empty() {
        let label =
            TextStyle::new(10.0, Rgba(156, 163, 175, 255)).weight(cosmic_text::Weight::SEMIBOLD);
        let lines = crate::ui::external_panel::draw::wrap(
            &mut painter,
            &message,
            layout.list.width() - 6.0,
            label,
        )
        .len();
        layout = Layout::new(kind, 0, width, lines as f32 * 15.0, bounds);
    }
    state.external_panel = ExternalPanel::open(
        kind,
        actions,
        message,
        layout,
        layout.position(anchor, bounds),
        monitor,
    );
    damage(state, dirty);
}

/// 【外部动作】【备用锚点】在 bounds 右侧返回原版备用工具栏范围，用于尚未完成面板定位的输入边界
fn fallback_rect(bounds: (f32, f32)) -> tiny_skia::Rect {
    tiny_skia::Rect::from_xywh(
        (bounds.0 - 64.0).max(0.0),
        (bounds.1 / 2.0 - 80.0).max(0.0),
        56.0,
        160.0,
    )
    .unwrap()
}

/// 【外部动作】【面板损坏】为 state 当前面板记录重绘范围，dirty 返回所属显示器，无返回值
fn damage(state: &mut EditorState, dirty: &mut u32) {
    if let Some(rect) = state.external_panel.rect() {
        emit_panel_damage(
            rect,
            state.external_panel.monitor_idx,
            &mut state.damage_rects,
            dirty,
        );
    }
}

/// 【外部动作】【列表事件】在 state 消费 event，dirty 返回重绘范围；动作先关闭面板，后由事件循环交付外部进程
fn event(state: &mut EditorState, event: Event, dirty: &mut u32) {
    match event {
        Event::None => {}
        Event::Changed => damage(state, dirty),
        Event::Close => close(state, dirty),
        Event::Activate(index) => {
            state.external_action_requested = state.external_panel.actions.get(index).cloned();
            close(state, dirty);
        }
    }
    apply_damage_rects(state, dirty);
}

/// 【外部动作】【指针隔离】在 state 打开面板时处理悬停并阻止下方工具移动，dirty 返回范围；返回是否消费输入
pub fn pointer(state: &mut EditorState, dirty: &mut u32) -> bool {
    if state.external_panel.kind.is_none() {
        return false;
    }
    let action = state
        .external_panel
        .pointer(state.input.pointer.monitor_idx, state.input.pointer.local);
    event(state, action, dirty);
    true
}

/// 【外部动作】【按钮隔离】在 state 中处理 button、pressed；左键面板动作、右键保护及工具栏切换保持独立，返回是否消费输入
pub fn button(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty: &mut u32,
) -> bool {
    let Some(kind) = state.external_panel.kind else {
        return false;
    };
    let monitor = state.input.pointer.monitor_idx;
    let point = state.input.pointer.local;
    let inside = state.external_panel.contains(monitor, point);
    let mut toolbar = None;
    for panel in [&state.toolbar, &state.action_toolbar] {
        if panel.monitor_idx == monitor
            && let (true, hit) = panel.hit_test(point)
        {
            toolbar = Some(hit.and_then(|index| panel.items.get(index)).copied());
        }
    }
    let same_toggle = matches!(
        (kind, toolbar),
        (
            Kind::OpenWith,
            Some(Some(ToolbarItem::Button(ToolbarButton::Integration(
                Integration::OpenWith
            ))))
        ) | (
            Kind::Extensions,
            Some(Some(ToolbarItem::Button(ToolbarButton::Integration(
                Integration::Extensions
            ))))
        )
    );
    if matches!(button, MouseButton::Left) && pressed && !inside {
        close(state, dirty);
        return toolbar.is_none() || same_toggle;
    }
    let action = state.external_panel.button(monitor, point, button, pressed);
    event(state, action, dirty);
    true
}

/// 【外部动作】【键盘隔离】把 chord 交给 state 当前列表，dirty 返回范围；返回是否消费输入，文字不进入已提交正文
pub fn key(state: &mut EditorState, chord: Option<crate::keys::Chord>, dirty: &mut u32) -> bool {
    if state.external_panel.kind.is_none() {
        return false;
    }
    if let Some(action) = chord.and_then(|chord| crate::keys::map().lookup(chord)) {
        let kind = match action {
            crate::keys::Action::OpenWith => Some(Kind::OpenWith),
            crate::keys::Action::Extensions => Some(Kind::Extensions),
            _ => None,
        };
        if let Some(kind) = kind {
            toggle(state, kind, dirty);
            return true;
        }
    }
    let action = chord.map_or(Event::None, |chord| state.external_panel.key(chord.key));
    event(state, action, dirty);
    true
}

/// 【外部动作】【滚轮隔离】按 scroll 滚动 state 的当前列表，dirty 返回范围；列表外滚轮同样不修改工具参数，返回是否消费输入
pub fn scroll(state: &mut EditorState, scroll: ScrollEvent, dirty: &mut u32) -> bool {
    if state.external_panel.kind.is_none() {
        return false;
    }
    if state
        .external_panel
        .contains(state.input.pointer.monitor_idx, state.input.pointer.local)
    {
        let action = state
            .external_panel
            .scroll_to(state.external_panel.offset + scroll.legacy_y());
        event(state, action, dirty);
    }
    true
}

/// 【外部动作】【进程交付】为 state 的 action 准备最终图片和全局区域；overlay 在需要关闭时释放输入，失败恢复原截图，返回是否已关闭
pub fn start(
    state: &mut EditorState,
    overlay: &mut dyn ScreenOverlay,
    action: Action,
) -> AppResult<bool> {
    let mut suspended = false;
    let result = (|| -> AppResult<_> {
        let image = if action.needs_image() {
            let (png, _, _) = super::render_final(state).ok_or("Cannot render screenshot")?;
            let png = crate::features::export_image::current(&png)?;
            Some(launch::temporary_image(&png, &std::env::temp_dir())?)
        } else {
            None
        };
        let geometry = state
            .selection
            .zone
            .map(|rect| {
                format!(
                    "{},{} {}x{}",
                    rect.left().floor() as i32,
                    rect.top().floor() as i32,
                    (rect.right().ceil() - rect.left().floor()) as i32,
                    (rect.bottom().ceil() - rect.top().floor()) as i32
                )
            })
            .unwrap_or_default();
        if action.close_on_start() {
            overlay.set_text_input(None)?;
            overlay.suspend()?;
            suspended = true;
        }
        let result = launch::start(&action, image.as_ref().map(|image| image.path()), &geometry);
        result?;
        // 1. 【外部动作】【文件交接】外部程序可能延迟读取，启动成功后文件独立存在；启动失败由句柄自动删除
        if let Some(image) = image {
            image.keep().map_err(|error| error.error)?;
        }
        Ok(action.close_on_start())
    })();
    match result {
        Ok(close) => {
            log::info!(
                "【外部动作】【启动完成】名称={}，关闭={close}",
                action.name()
            );
            Ok(close)
        }
        Err(error) => {
            if suspended {
                overlay.resume()?;
                for (index, capture) in state.captures.iter().enumerate() {
                    overlay.set_background(
                        index,
                        capture.pixmap.data(),
                        capture.pixmap.width(),
                        capture.pixmap.height(),
                    )?;
                }
            }
            log::error!(
                "【外部动作】【启动失败】名称={}，错误={error}",
                action.name()
            );
            state.toasts.show_text(
                crate::ui::toast::ToastKind::ExternalActionFailed,
                format!("Cannot start {}: {error}", action.name()),
                &mut state.text.font_system,
            );
            Ok(false)
        }
    }
}
