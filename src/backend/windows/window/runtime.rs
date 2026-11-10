//! 【Windows独立窗口】【线程运行】窗口、菜单和模态内容始终在创建线程消费事件

use super::super::{input::Decoder, native};
use super::popup::{Popup, popup_event, sync_popups};
use crate::{
    backend::window::{WindowAction, WindowContent, WindowCursor, WindowEvent, WindowSpec},
    theme::config_watch::Watch,
};
use std::time::Instant;
use tiny_skia::Pixmap;

pub(super) struct Surface {
    pub(super) window: native::Window,
    pub(super) content: Box<dyn WindowContent>,
    canvas: Pixmap,
    pub(super) scale: f32,
    dirty: bool,
    decoder: Decoder,
    focus_id: u64,
    text: Option<crate::types::text_input::Snapshot>,
}

impl Surface {
    /// 【Windows独立窗口】【内容创建】按spec、content、queue与owner创建隐藏表面，返回已交付尺寸事件的内容窗口
    fn new(
        spec: WindowSpec,
        mut content: Box<dyn WindowContent>,
        queue: native::EventQueue,
        owner: isize,
    ) -> crate::AppResult<Self> {
        let monitors = native::monitors()?;
        let monitor = monitors
            .iter()
            .find(|monitor| monitor.primary)
            .unwrap_or(&monitors[0]);
        let scale = monitor.dpi as f32 / 96.0;
        let size = (
            (spec.size.0 as f32 * scale).round().max(1.0) as u32,
            (spec.size.1 as f32 * scale).round().max(1.0) as u32,
        );
        let mut bounds = native::Bounds {
            left: monitor.bounds.left + (monitor.bounds.width.saturating_sub(size.0) / 2) as i32,
            top: monitor.bounds.top + (monitor.bounds.height.saturating_sub(size.1) / 2) as i32,
            width: size.0,
            height: size.1,
        };
        if let Some((left, top)) = content.initial_position() {
            bounds.left = left;
            bounds.top = top;
        }
        content.stacking_supported(true);
        let window = native::Window::new(
            native::WindowOptions {
                title: spec.title,
                bounds,
                minimum: spec.minimum,
                topmost: content.always_on_top(),
                activate: true,
                layered: true,
                owner,
            },
            queue,
        )?;
        content.event(WindowEvent::Resize { size: spec.size });
        let canvas = Pixmap::new(size.0, size.1).ok_or("cannot allocate Windows content frame")?;
        Ok(Self {
            window,
            content,
            canvas,
            scale,
            dirty: true,
            decoder: Decoder::default(),
            focus_id: crate::features::text::input_method::focus::identity(),
            text: None,
        })
    }
    /// 【Windows独立窗口】【输入快照】无参数，同步当前编辑目标和物理光标，目标消失时取消候选，返回系统结果
    fn sync_text(&mut self) -> crate::AppResult {
        let Some(target_id) = self.content.text_input_target() else {
            self.text = None;
            self.window.set_text_input(0, None)?;
            return Ok(());
        };
        let focus = crate::types::text_input::Focus {
            id: self.focus_id,
            monitor_idx: 0,
        };
        if let Some(snapshot) = self.content.text_input_snapshot(
            focus,
            target_id,
            self.text.as_ref(),
            crate::types::text_input::ChangeCause::Other,
        ) {
            let rect = snapshot.cursor_rect.map(|(x, y, w, h)| {
                (
                    (x as f32 * self.scale).round() as i32,
                    (y as f32 * self.scale).round() as i32,
                    (w as f32 * self.scale).ceil().max(1.0) as i32,
                    (h as f32 * self.scale).ceil().max(1.0) as i32,
                )
            });
            self.window.set_text_input(self.focus_id, rect)?;
            self.text = Some(snapshot);
        }
        Ok(())
    }
    /// 【Windows独立窗口】【内容呈现】无参数，在尺寸或内容变化时提交共享绘制，返回系统结果，不安排空闲绘制
    fn paint(&mut self) -> crate::AppResult {
        if !self.dirty {
            return Ok(());
        }
        let mut bounds = self.window.bounds()?;
        let mut geometry_changed = false;
        if let Some((x, y)) = self.content.take_position_offset()
            && x.is_finite()
            && y.is_finite()
        {
            bounds.left = bounds
                .left
                .saturating_add((x * f64::from(self.scale)).round() as i32);
            bounds.top = bounds
                .top
                .saturating_add((y * f64::from(self.scale)).round() as i32);
            geometry_changed = true;
        }
        if let Some(size) = self.content.preferred_size() {
            let physical = (
                (size.0 as f32 * self.scale).round().max(1.0) as u32,
                (size.1 as f32 * self.scale).round().max(1.0) as u32,
            );
            if (bounds.width, bounds.height) != physical {
                bounds.width = physical.0;
                bounds.height = physical.1;
                geometry_changed = true;
            }
        }
        if geometry_changed {
            self.window.set_bounds(bounds)?;
        }
        if (self.canvas.width(), self.canvas.height()) != (bounds.width, bounds.height) {
            self.canvas = Pixmap::new(bounds.width, bounds.height)
                .ok_or("cannot allocate resized Windows content frame")?;
        }
        self.canvas.fill(tiny_skia::Color::TRANSPARENT);
        self.content.draw(&mut self.canvas, self.scale);
        self.window.set_topmost(self.content.always_on_top())?;
        self.window.set_rgba(
            (self.canvas.width(), self.canvas.height()),
            self.canvas.data(),
        )?;
        self.dirty = false;
        Ok(())
    }
    /// 【Windows独立窗口】【动作应用】按action调整窗口或重绘，返回是否关闭，移动缩放交给系统并保持事件收集
    pub(super) fn apply(&mut self, action: WindowAction) -> bool {
        match action {
            WindowAction::Close => return true,
            WindowAction::Redraw => self.dirty = true,
            WindowAction::Move | WindowAction::Resize | WindowAction::ResizeEdge(_) => {
                use crate::backend::window::WindowEdge;
                use windows_sys::Win32::UI::{
                    Input::KeyboardAndMouse::ReleaseCapture,
                    WindowsAndMessaging::{
                        HTCAPTION, SC_MOVE, SC_SIZE, SendMessageW, WM_SYSCOMMAND, WMSZ_BOTTOM,
                        WMSZ_BOTTOMLEFT, WMSZ_BOTTOMRIGHT, WMSZ_LEFT, WMSZ_RIGHT, WMSZ_TOP,
                        WMSZ_TOPLEFT, WMSZ_TOPRIGHT,
                    },
                };
                let edge = match action {
                    WindowAction::ResizeEdge(WindowEdge::Top) => WMSZ_TOP,
                    WindowAction::ResizeEdge(WindowEdge::Bottom) => WMSZ_BOTTOM,
                    WindowAction::ResizeEdge(WindowEdge::Left) => WMSZ_LEFT,
                    WindowAction::ResizeEdge(WindowEdge::Right) => WMSZ_RIGHT,
                    WindowAction::ResizeEdge(WindowEdge::TopLeft) => WMSZ_TOPLEFT,
                    WindowAction::ResizeEdge(WindowEdge::TopRight) => WMSZ_TOPRIGHT,
                    WindowAction::ResizeEdge(WindowEdge::BottomLeft) => WMSZ_BOTTOMLEFT,
                    _ => WMSZ_BOTTOMRIGHT,
                };
                unsafe {
                    ReleaseCapture();
                    SendMessageW(
                        self.window.handle() as _,
                        WM_SYSCOMMAND,
                        (if action == WindowAction::Move {
                            SC_MOVE | HTCAPTION
                        } else {
                            SC_SIZE | edge
                        }) as usize,
                        0,
                    );
                }
            }
            _ => {}
        }
        false
    }
    /// 【Windows独立窗口】【消息转换】按event更新DPI、内容与光标，返回是否关闭，过期窗口编号由运行时过滤
    fn event(&mut self, event: native::Event) -> crate::AppResult<bool> {
        if matches!(event, native::Event::Close { .. }) {
            let close = self.content.close_requested();
            self.dirty |= !close;
            return Ok(close);
        }
        if let native::Event::Composition { target, .. } = &event
            && (self.text.is_none() || *target != self.focus_id)
        {
            return Ok(false);
        }
        if matches!(event, native::Event::Focus { focused: true, .. }) {
            self.focus_id = crate::features::text::input_method::focus::identity();
            self.text = None;
        }
        if let native::Event::Dpi { dpi, bounds, .. } = event {
            self.scale = dpi as f32 / 96.0;
            self.window.set_bounds(bounds)?;
            let action = self.content.event(WindowEvent::Resize {
                size: (
                    (bounds.width as f32 / self.scale).round().max(1.0) as u32,
                    (bounds.height as f32 / self.scale).round().max(1.0) as u32,
                ),
            });
            self.dirty = true;
            return Ok(self.apply(action));
        }
        let Some(event) = self.decoder.event(event, self.scale) else {
            return Ok(false);
        };
        if let WindowEvent::Key(input) = &event
            && super::theme_input::handle(
                self,
                input.key.map(|key| crate::keys::Chord {
                    key,
                    mods: input.mods,
                }),
            )?
        {
            return Ok(false);
        }
        if let WindowEvent::PointerMove { position } = event {
            let cursor = match self.content.cursor(position) {
                WindowCursor::Text => native::Cursor::Text,
                WindowCursor::Pointer => native::Cursor::Pointer,
                WindowCursor::Grab | WindowCursor::Grabbing => native::Cursor::Move,
                WindowCursor::Resize => native::Cursor::NwSeResize,
                WindowCursor::HorizontalResize => native::Cursor::HorizontalResize,
                WindowCursor::VerticalResize => native::Cursor::VerticalResize,
                WindowCursor::Default => native::Cursor::Default,
            };
            self.window.set_cursor(cursor)?;
        }
        let lowering = self.content.always_on_top() && matches!(event, WindowEvent::Key(_));
        if lowering {
            self.window.set_topmost(false)?;
        }
        let action = match event {
            WindowEvent::Wheel(input) => self.content.scroll(input),
            event => self.content.event(event),
        };
        if lowering {
            self.window.set_topmost(self.content.always_on_top())?;
        }
        Ok(self.apply(action))
    }
}

/// 【Windows独立窗口】【完整运行】按spec与content驱动原生窗口、模态子窗口和菜单，返回退出或系统错误，所有资源在当前线程回收
pub fn run(spec: WindowSpec, content: impl WindowContent + 'static) -> crate::AppResult {
    let queue = native::EventQueue::default();
    let wake = native::Wake::new()?;
    let theme = Watch::current();
    let mut main = Surface::new(spec, Box::new(content), queue.clone(), 0)?;
    main.content.connected(wake.clone());
    main.paint()?;
    main.content.mapped();
    let mut dialog: Option<(u64, Surface)> = None;
    let visibility_wake = wake.clone();
    let mut visibility =
        crate::backend::capture_visibility::Watch::new(move || visibility_wake.notify())?;
    let mut hidden = false;
    let mut popups = Vec::<Popup>::new();
    loop {
        let requested = visibility.requested()?;
        if requested != hidden {
            if requested {
                popups.clear();
                main.window.hide();
                if let Some((_, child)) = &mut dialog {
                    child.window.hide();
                }
            }
            main.dirty = true;
            if let Some((_, child)) = &mut dialog {
                child.dirty = true;
            }
            hidden = requested;
        }
        visibility.applied()?;
        if hidden {
            native::dispatch(
                &[wake.handle()],
                Some(std::time::Duration::from_millis(100)),
            )?;
            continue;
        }
        let active = dialog.as_mut().map_or(&mut main, |entry| &mut entry.1);
        active.sync_text()?;
        sync_popups(active, &mut popups, &queue)?;
        main.paint()?;
        if let Some((_, surface)) = &mut dialog {
            surface.paint()?;
        }
        if dialog.is_none()
            && let Some(request) = main.content.take_dialog()
        {
            popups.clear();
            main.content.cancel_text_input();
            let mut child = Surface::new(
                request.spec,
                request.content,
                queue.clone(),
                main.window.handle(),
            )?;
            child.content.connected(wake.clone());
            main.window.set_enabled(false);
            child.paint()?;
            child.content.mapped();
            dialog = Some((request.id, child));
        }
        if let Some(watch) = &theme {
            match watch.changed() {
                Ok(Some(mode)) => {
                    crate::theme::apply_saved(mode);
                    main.dirty = true;
                    if let Some((_, child)) = &mut dialog {
                        child.dirty = true;
                    }
                }
                Ok(None) => {}
                Err(error) => log::error!("【Windows独立窗口】【主题读取】{error}"),
            }
        }
        let deadline = [
            main.content.next_deadline(),
            dialog
                .as_ref()
                .and_then(|entry| entry.1.content.next_deadline()),
        ]
        .into_iter()
        .flatten()
        .min();
        let timeout = deadline.map(|at| at.saturating_duration_since(Instant::now()));
        let mut handles = vec![wake.handle()];
        if let Some(watch) = &theme {
            handles.push(watch.handle());
        }
        let wait = native::dispatch(&handles, timeout)?;
        if matches!(wait, native::Wait::Source(0) | native::Wait::Timeout) {
            let action = main.content.event(WindowEvent::Wake);
            if main.apply(action) {
                return Ok(());
            }
            if let Some((_, child)) = &mut dialog {
                let action = child.content.event(WindowEvent::Wake);
                child.apply(action);
            }
        }
        while let Some(event) = queue.pop() {
            let active = dialog.as_mut().map_or(&mut main, |entry| &mut entry.1);
            if let Some(index) = popups
                .iter()
                .position(|popup| popup.window.id() == event.id())
            {
                let id = popups[index].id;
                let event = popups[index]
                    .decoder
                    .event(event, active.scale)
                    .and_then(popup_event);
                let Some(event) = event else {
                    continue;
                };
                if super::theme_input::popup(active, &event)? {
                    continue;
                }
                let action = active.content.popup_event(id, event);
                if super::popup::apply(active, &mut popups, id, action) {
                    return Ok(());
                }
                continue;
            }
            if event.id() != active.window.id() {
                continue;
            }
            if !popups.is_empty() {
                if matches!(event, native::Event::Button { pressed: true, .. }) {
                    let id = popups[0].id;
                    if super::popup::finish(active, &mut popups, id, None) {
                        return Ok(());
                    }
                    continue;
                }
                if matches!(
                    event,
                    native::Event::Key { .. } | native::Event::Character { .. }
                ) {
                    let id = popups.last().unwrap().id;
                    if let Some(event) = active
                        .decoder
                        .event(event, active.scale)
                        .and_then(popup_event)
                    {
                        if super::theme_input::popup(active, &event)? {
                            continue;
                        }
                        let action = active.content.popup_event(id, event);
                        if super::popup::apply(active, &mut popups, id, action) {
                            return Ok(());
                        }
                    }
                    continue;
                }
            }
            if active.event(event)? {
                popups.clear();
                if let Some((id, child)) = dialog.take() {
                    let accepted = child.content.dialog_accepted();
                    drop(child);
                    main.window.set_enabled(true);
                    let action = main.content.dialog_closed(id, accepted);
                    if main.apply(action) {
                        return Ok(());
                    }
                } else {
                    return Ok(());
                }
            }
        }
    }
}
