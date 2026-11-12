//! 【X11普通窗口】【内容运行时】设置、OCR和贴图共享原生输入、模态窗口及菜单生命周期

mod capture_visibility;
mod events;
mod geometry;
mod popup;
mod text_input;

use super::{connection::Display, surface::Surface};
use crate::backend::{task_wake, window::*};
use tiny_skia::Pixmap;
use x11rb::{
    connection::Connection,
    protocol::{Event, xproto::*},
    wrapper::ConnectionExt as _,
};

use geometry::Drag;

struct Node {
    surface: Surface,
    content: Box<dyn WindowContent>,
    spec: WindowSpec,
    dirty: bool,
    close: bool,
    popups: Vec<(u64, Surface)>,
    popup_grab: Option<super::popup_grab::Grab>,
    dialog: Option<(u64, Box<Node>)>,
    topmost: Option<bool>,
    pointer: (f64, f64),
    drag: Option<Drag>,
    keyboard: super::input::Keyboard,
    ime: super::input_method::InputMethod,
    focus: Option<crate::types::text_input::Focus>,
    text_snapshot: Option<crate::types::text_input::Snapshot>,
    text_cause: crate::types::text_input::ChangeCause,
    cursor: Option<crate::types::CursorIcon>,
}

impl Node {
    /// 【X11普通窗口】【内容连接】按display、spec、content及wake创建未映射内容窗口，返回初始化结果
    fn new(
        display: Display,
        spec: WindowSpec,
        mut content: Box<dyn WindowContent>,
        wake: &task_wake::Source,
    ) -> crate::AppResult<Self> {
        let size = (
            spec.size.0.max(spec.minimum.0),
            spec.size.1.max(spec.minimum.1),
        );
        let position = content.initial_position().unwrap_or((
            (i32::from(display.screen().width_in_pixels) - size.0 as i32).max(0) / 2,
            (i32::from(display.screen().height_in_pixels) - size.1 as i32).max(0) / 2,
        ));
        let surface = Surface::new(display.clone(), position, size, false)?;
        surface.title(&spec.title)?;
        let mut hints = [0u32; 18];
        hints[0] = (1 << 2) | (1 << 4);
        hints[1] = position.0 as u32;
        hints[2] = position.1 as u32;
        hints[5] = spec.minimum.0;
        hints[6] = spec.minimum.1;
        display.connection.change_property32(
            PropMode::REPLACE,
            surface.window,
            AtomEnum::WM_NORMAL_HINTS,
            AtomEnum::WM_SIZE_HINTS,
            &hints,
        )?;
        display.connection.change_property32(
            PropMode::REPLACE,
            surface.window,
            display.atom("_MOTIF_WM_HINTS")?,
            display.atom("_MOTIF_WM_HINTS")?,
            &[2, 0, 0, 0, 0],
        )?;
        content.stacking_supported(
            display
                .property(display.screen().root, "_NET_SUPPORTED")?
                .contains(&display.atom("_NET_WM_STATE_ABOVE")?),
        );
        content.connected(WindowWake::x11(wake.wake()));
        content.event(WindowEvent::Resize { size });
        Ok(Self {
            keyboard: super::input::Keyboard::new(&display)?,
            ime: super::input_method::InputMethod::new(&display),
            focus: None,
            text_snapshot: None,
            text_cause: crate::types::text_input::ChangeCause::Other,
            cursor: None,
            surface,
            content,
            spec,
            dirty: true,
            close: false,
            popups: Vec::new(),
            popup_grab: None,
            dialog: None,
            topmost: None,
            pointer: (0.0, 0.0),
            drag: None,
        })
    }

    /// 【X11普通窗口】【动作交付】按action更新绘制、关闭或窗口管理状态，无返回值
    fn action(&mut self, action: WindowAction) {
        match action {
            WindowAction::Redraw => self.dirty = true,
            WindowAction::Close => self.close = true,
            WindowAction::Move | WindowAction::Resize | WindowAction::ResizeEdge(_) => {
                if let Err(error) = self.begin_drag(action) {
                    log::error!("【X11普通窗口】【移动缩放】{error}");
                }
            }
            WindowAction::None => {}
        }
    }

    /// 【X11普通窗口】【状态同步】按wake同步尺寸、置顶、模态窗口和变化像素，返回协议错误
    fn sync(&mut self, wake: &task_wake::Source) -> crate::AppResult {
        let display = self.surface.display.clone();
        use crate::types::CursorIcon;
        let cursor = match self.content.cursor(self.pointer) {
            WindowCursor::Default => CursorIcon::Default,
            WindowCursor::Text => CursorIcon::Text,
            WindowCursor::Pointer => CursorIcon::Pointer,
            WindowCursor::Grab => CursorIcon::Grab,
            WindowCursor::Grabbing => CursorIcon::Grabbing,
            WindowCursor::Resize => CursorIcon::NwseResize,
            WindowCursor::HorizontalResize => CursorIcon::EwResize,
            WindowCursor::VerticalResize => CursorIcon::NsResize,
        };
        if self.cursor != Some(cursor) {
            super::cursor::set(&display, [self.surface.window], cursor)?;
            self.cursor = Some(cursor);
        }
        if let Some(size) = self.content.preferred_size() {
            let size = (
                size.0.max(self.spec.minimum.0),
                size.1.max(self.spec.minimum.1),
            );
            let native = (u16::try_from(size.0)?, u16::try_from(size.1)?);
            if native != self.surface.size {
                display
                    .connection
                    .configure_window(
                        self.surface.window,
                        &ConfigureWindowAux::new().width(size.0).height(size.1),
                    )?
                    .check()?;
                self.surface.size = native;
                self.content.event(WindowEvent::Resize { size });
                self.dirty = true;
            }
        }
        if let Some((offset_x, offset_y)) = self.content.take_position_offset() {
            let position = (
                i32::from(self.surface.position.0) + offset_x.round() as i32,
                i32::from(self.surface.position.1) + offset_y.round() as i32,
            );
            display
                .connection
                .configure_window(
                    self.surface.window,
                    &ConfigureWindowAux::new().x(position.0).y(position.1),
                )?
                .check()?;
            self.surface.position = (position.0.try_into()?, position.1.try_into()?);
        }
        let topmost = self.content.always_on_top();
        if self.topmost != Some(topmost) {
            let state = display.atom("_NET_WM_STATE")?;
            let above = display.atom("_NET_WM_STATE_ABOVE")?;
            display.connection.send_event(
                false,
                display.screen().root,
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                ClientMessageEvent::new(
                    32,
                    self.surface.window,
                    state,
                    [u32::from(topmost), above, 0, 1, 0],
                ),
            )?;
            if !self.surface.mapped {
                display.connection.change_property32(
                    PropMode::REPLACE,
                    self.surface.window,
                    state,
                    AtomEnum::ATOM,
                    if topmost {
                        std::slice::from_ref(&above)
                    } else {
                        &[]
                    },
                )?;
            }
            self.topmost = Some(topmost);
        }
        if self.dirty {
            let mut frame = Pixmap::new(self.surface.size.0.into(), self.surface.size.1.into())
                .ok_or("X11 window allocation failed")?;
            self.content.draw(&mut frame, 1.0);
            let first = !self.surface.mapped;
            self.surface.present(frame.data())?;
            self.dirty = false;
            if first {
                self.content.mapped();
            }
        }
        self.sync_popups()?;
        if self.dialog.is_none()
            && let Some(dialog) = self.content.take_dialog()
        {
            let child = Self::new(display.clone(), dialog.spec, dialog.content, wake)?;
            display.connection.change_property32(
                PropMode::REPLACE,
                child.surface.window,
                AtomEnum::WM_TRANSIENT_FOR,
                AtomEnum::WINDOW,
                &[self.surface.window],
            )?;
            display.connection.change_property32(
                PropMode::REPLACE,
                child.surface.window,
                display.atom("_NET_WM_STATE")?,
                AtomEnum::ATOM,
                &[display.atom("_NET_WM_STATE_MODAL")?],
            )?;
            self.dialog = Some((dialog.id, Box::new(child)));
        }
        if let Some((id, child)) = &mut self.dialog {
            child.sync(wake)?;
            if child.close {
                let (id, accepted) = (*id, child.content.dialog_accepted());
                self.dialog = None;
                let action = self.content.dialog_closed(id, accepted);
                self.action(action);
            }
        }
        self.sync_text_input()?;
        Ok(())
    }

    /// 【X11普通窗口】【后台唤醒】无参数，将完成通知交给当前内容及子窗口，无返回值
    fn wake(&mut self) {
        let action = self.content.event(WindowEvent::Wake);
        self.action(action);
        if let Some((_, child)) = &mut self.dialog {
            child.wake();
        }
    }

    /// 【X11普通窗口】【主题刷新】无参数，标记所有存活内容重绘，菜单保留当前交互状态
    fn refresh_theme(&mut self) {
        self.dirty = true;
        if let Some((_, child)) = &mut self.dialog {
            child.refresh_theme();
        }
    }

    /// 【X11普通窗口】【最近期限】无参数，返回本内容及子窗口最近的一次性唤醒期限
    fn deadline(&self) -> Option<std::time::Instant> {
        [
            self.content.next_deadline(),
            self.dialog.as_ref().and_then(|(_, child)| child.deadline()),
        ]
        .into_iter()
        .flatten()
        .min()
    }
}

/// 【X11普通窗口】【阻塞运行】按display、spec和content驱动原生窗口至关闭，返回协议或绘制错误
pub fn run(
    display: Display,
    spec: WindowSpec,
    content: impl WindowContent + 'static,
) -> crate::AppResult {
    let wake = task_wake::Source::new()?;
    let theme = crate::theme::config_watch::Watch::current();
    let mut node = Node::new(display.clone(), spec, Box::new(content), &wake)?;
    let visibility_wake = wake.wake();
    let mut visibility =
        crate::backend::capture_visibility::Watch::new(move || visibility_wake.notify())?;
    let mut hidden = false;
    let mut mode = crate::theme::Mode::current();
    loop {
        if super::wait::theme_changed(theme.as_ref()) || mode != crate::theme::Mode::current() {
            mode = crate::theme::Mode::current();
            node.refresh_theme();
        }
        let requested = visibility.requested()?;
        if requested != hidden {
            node.capture_visibility(requested)?;
            hidden = requested;
        }
        visibility.applied()?;
        if !hidden {
            node.sync(&wake)?;
        }
        if node.close {
            return Ok(());
        }
        let mut received = false;
        while let Some(event) = display.connection.poll_for_event()? {
            node.event(event)?;
            received = true;
        }
        if received {
            continue;
        }
        let deadline = [node.deadline(), visibility.deadline()]
            .into_iter()
            .flatten()
            .min();
        let timeout =
            deadline.map(|deadline| deadline.saturating_duration_since(std::time::Instant::now()));
        let notified = super::wait::wait_with_theme(&display, &wake, timeout, theme.as_ref())?;
        if notified || deadline.is_some_and(|deadline| deadline <= std::time::Instant::now()) {
            node.wake();
        }
    }
}
