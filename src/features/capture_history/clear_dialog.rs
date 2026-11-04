//! 【截图历史】【清空对话框】独立模态窗口的焦点、默认取消和按下释放事务

use super::clear_dialog_draw::Layout;
use crate::{
    backend::wayland::window::{
        KeyInput, LEFT_BUTTON, RIGHT_BUTTON, WindowAction, WindowContent, WindowDialog,
        WindowEvent, WindowSpec,
    },
    keys::{Key, Mods},
    ui::fonts::UiFonts,
};
use cosmic_text::SwashCache;
use tiny_skia::Pixmap;

pub(super) const MESSAGE: &str = "Delete all screenshot history? Saved files will be kept.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Button {
    Cancel,
    Clear,
}

impl Button {
    /// 【截图历史】【确认文本】返回本按钮的原版标签，不使用图标或额外说明，无参数
    pub fn label(self) -> &'static str {
        match self {
            Self::Cancel => "Cancel",
            Self::Clear => "Clear History",
        }
    }
}

pub(super) struct ClearDialog {
    pub fonts: UiFonts,
    pub cache: SwashCache,
    pub layout: Layout,
    pub focus: Button,
    pub focused: bool,
    pub hover: Option<Button>,
    pub pressed: Option<Button>,
    keyboard_pressed: bool,
    accepted: bool,
}

impl ClearDialog {
    /// 【截图历史】【对话框构造】返回默认取消的只读确认内容，不读取历史或初始化 OCR，无参数
    pub fn new() -> Self {
        Self {
            fonts: UiFonts::with_system_family("DejaVu LGC Sans"),
            cache: SwashCache::new(),
            layout: Layout::new((315, 96)),
            focus: Button::Cancel,
            focused: true,
            hover: None,
            pressed: None,
            keyboard_pressed: false,
            accepted: false,
        }
    }

    /// 【截图历史】【关闭结果】按 button 保存明确结果，返回关闭本对话框的请求，不直接删除文件
    fn close(&mut self, button: Button) -> WindowAction {
        self.accepted = button == Button::Clear;
        self.pressed = None;
        self.keyboard_pressed = false;
        WindowAction::Close
    }

    /// 【截图历史】【确认焦点】把焦点移到 button，变化时取消旧按钮事务，返回是否变化
    fn focus(&mut self, button: Button) -> bool {
        if self.focus == button {
            return false;
        }
        self.focus = button;
        self.pressed = None;
        self.keyboard_pressed = false;
        true
    }

    /// 【截图历史】【确认键盘】处理 input，Enter 使用当前默认按钮，Space 等待释放，返回窗口动作
    fn key(&mut self, input: KeyInput) -> WindowAction {
        if input.key == Some(Key::Escape) && input.mods == Mods::default() {
            return self.close(Button::Cancel);
        }
        if input.key == Some(Key::Enter) {
            return self.close(self.focus);
        }
        if input.key == Some(Key::Space) {
            self.pressed = Some(self.focus);
            self.keyboard_pressed = true;
            return WindowAction::Redraw;
        }
        if input.mods.ctrl || input.mods.alt || input.mods.logo {
            return WindowAction::None;
        }
        match input.key {
            Some(Key::Tab | Key::Left | Key::Right | Key::Up | Key::Down) => {
                self.focus(if self.focus == Button::Cancel {
                    Button::Clear
                } else {
                    Button::Cancel
                });
                WindowAction::Redraw
            }
            _ => WindowAction::None,
        }
    }
}

impl WindowContent for ClearDialog {
    /// 【截图历史】【确认绘制】把本对话框按 scale 绘制到 canvas，复用统一控件，不绘制父窗口或额外标题
    fn draw(&mut self, canvas: &mut Pixmap, scale: f32) {
        super::clear_dialog_draw::draw(self, canvas, scale);
    }

    /// 【截图历史】【确认事件】按 event 更新只读焦点和释放事务，返回重绘或本窗口关闭，不处理父窗口快捷键
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        match event {
            WindowEvent::Resize { size } => {
                self.layout = Layout::new(size);
                WindowAction::Redraw
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                if !focused {
                    self.pressed = None;
                    self.keyboard_pressed = false;
                }
                WindowAction::Redraw
            }
            WindowEvent::Key(input) => self.key(input),
            WindowEvent::KeyRelease(input)
                if input.key == Some(Key::Space) && self.keyboard_pressed =>
            {
                self.keyboard_pressed = false;
                match self.pressed.take() {
                    Some(button) if button == self.focus => self.close(button),
                    _ => WindowAction::Redraw,
                }
            }
            WindowEvent::PointerButton {
                position,
                button,
                pressed: true,
            } if matches!(button, LEFT_BUTTON | RIGHT_BUTTON) => {
                let Some(control) = self.layout.button(position) else {
                    return WindowAction::None;
                };
                let changed = self.focus(control);
                if button == LEFT_BUTTON {
                    self.pressed = Some(control);
                    self.keyboard_pressed = false;
                    return WindowAction::Redraw;
                }
                if changed {
                    WindowAction::Redraw
                } else {
                    WindowAction::None
                }
            }
            WindowEvent::PointerButton {
                position,
                button: LEFT_BUTTON,
                pressed: false,
            } if !self.keyboard_pressed => match self.pressed.take() {
                Some(button) if self.layout.button(position) == Some(button) => self.close(button),
                Some(_) => WindowAction::Redraw,
                None => WindowAction::None,
            },
            WindowEvent::PointerMove { position } => {
                let hover = self.layout.button(position);
                if self.hover == hover {
                    WindowAction::None
                } else {
                    self.hover = hover;
                    WindowAction::Redraw
                }
            }
            WindowEvent::PointerLeave => {
                if self.hover.take().is_some() {
                    WindowAction::Redraw
                } else {
                    WindowAction::None
                }
            }
            _ => WindowAction::None,
        }
    }

    /// 【截图历史】【明确确认】返回本窗口是否选择清空；窗口管理器关闭由后端统一按取消处理，无参数
    fn dialog_accepted(&self) -> bool {
        self.accepted
    }
}

/// 【截图历史】【子窗口请求】为 id 返回原版尺寸、标题和只读模态内容，业务结果只交回父窗口
pub(super) fn request(id: u64) -> WindowDialog {
    WindowDialog {
        id,
        spec: WindowSpec {
            title: "Clear History".into(),
            app_id: "mark-shot-history-clear".into(),
            size: (315, 96),
            minimum: (223, 77),
        },
        content: Box::new(ClearDialog::new()),
    }
}

#[cfg(test)]
#[path = "clear_dialog_tests.rs"]
mod tests;
