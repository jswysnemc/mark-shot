//! 【扫码窗口】【只读交互】选择、复制和滚动可用，不执行识别内容中的链接或指令

use super::{
    ResultWindow,
    layout::{Control, Layout, contains},
};
use crate::{
    backend::window::{self, LEFT_BUTTON, WindowAction, WindowContent, WindowCursor, WindowEvent},
    keys::Key,
    ui::text_editor::EditEvent,
};

impl ResultWindow {
    /// 【扫码窗口】【复制结果】按 selected 复制所选文字或全部正文，返回值为空，失败显示真实错误
    fn copy(&mut self, selected: bool) {
        let text = if selected {
            self.editor.selected_text()
        } else {
            Some(self.editor.text())
        };
        if let Some(text) = text.filter(|text| !text.is_empty()) {
            self.status = match crate::utils::copy_to_clipboard(&text) {
                Ok(()) => if self.chinese { "已复制" } else { "Copied" }.into(),
                Err(error) => error.to_string(),
            };
        }
    }
}

impl WindowContent for ResultWindow {
    /// 【扫码窗口】【置顶来源】无参数，返回贴图交接要求的置顶状态，普通扫码保持普通窗口
    fn always_on_top(&self) -> bool {
        self.above_pin
    }

    /// 【扫码窗口】【贴图避让】无参数，置顶结果使用高于贴图的浮层，避免原图挡住正文和复制按钮
    fn above_pinned_windows(&self) -> bool {
        self.above_pin
    }

    /// 【扫码窗口】【任务接入】按 wake 启动后台识别，结果完成后唤醒原生窗口
    fn connected(&mut self, wake: window::WindowWake) {
        self.start(wake);
    }

    /// 【扫码窗口】【首帧确认】无参数，通知交接进程窗口已显示，此后可关闭截图覆盖层
    fn mapped(&mut self) {
        if self.ready {
            use std::io::Write;
            let _ = std::io::stdout().write_all(b"\n");
            self.ready = false;
        }
    }

    /// 【扫码窗口】【绘制委托】按 canvas 与 scale 绘制当前状态，返回值为空
    fn draw(&mut self, canvas: &mut tiny_skia::Pixmap, scale: f32) {
        super::draw::draw(self, canvas, scale);
    }

    /// 【扫码窗口】【光标选择】按 position 返回正文选择或默认光标，不启用输入法
    fn cursor(&self, position: (f64, f64)) -> WindowCursor {
        if contains(Layout::new(self.size).text, position) {
            WindowCursor::Text
        } else {
            WindowCursor::Default
        }
    }

    /// 【扫码窗口】【事件分派】按 event 处理只读文字选择、滚动条及按钮，返回重绘或关闭动作
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        let layout = Layout::new(self.size);
        match event {
            WindowEvent::Wake => self.poll(),
            WindowEvent::Resize { size } => self.size = size,
            WindowEvent::Focused(false) | WindowEvent::PointerLeave => {
                self.dragging = false;
                self.scroll_drag = None;
                self.pressed = None;
            }
            WindowEvent::Key(input) => {
                if input.key == Some(Key::Escape) {
                    return WindowAction::Close;
                }
                // 1. 【扫码窗口】【只读按键】仅转交移动、全选和复制，剪切粘贴等编辑指令不进入控件
                let navigation = matches!(
                    input.key,
                    Some(
                        Key::Left
                            | Key::Right
                            | Key::Up
                            | Key::Down
                            | Key::Home
                            | Key::End
                            | Key::PageUp
                            | Key::PageDown
                    )
                );
                let shortcut = input.mods.ctrl
                    && !input.mods.alt
                    && !input.mods.logo
                    && matches!(input.key, Some(Key::Char('a' | 'c')));
                if (navigation || shortcut)
                    && let EditEvent::Copy(_) = self.editor.key(&mut self.fonts, &input)
                {
                    self.copy(!input.mods.shift);
                }
            }
            WindowEvent::PointerButton {
                position,
                button: LEFT_BUTTON,
                pressed,
            } => {
                if !pressed {
                    self.dragging = false;
                    self.scroll_drag = None;
                    let hit = layout.hit(position);
                    if self.pressed.take() == hit
                        && let Some(control) = hit
                    {
                        match control {
                            Control::Close => return WindowAction::Close,
                            Control::Copy => self.copy(true),
                            Control::CopyAll => self.copy(false),
                        }
                    }
                } else {
                    self.pressed = layout.hit(position);
                    if let Some(bar) = self.editor.scrollbar(&mut self.fonts, layout.viewport)
                        && contains(bar.track, position)
                    {
                        let anchor = if contains(bar.handle, position) {
                            position
                        } else {
                            (
                                position.0,
                                f64::from(bar.handle.y() + bar.handle.height() / 2.0),
                            )
                        };
                        let drag = crate::ui::scrollbar::Drag::new(anchor, &bar);
                        self.editor
                            .scroll_to(&mut self.fonts, drag.value(position, &bar));
                        self.scroll_drag = Some(drag);
                    } else if contains(layout.text, position) {
                        self.dragging = true;
                        self.editor.pointer(
                            &mut self.fonts,
                            (
                                position.0 - f64::from(layout.text.x()),
                                position.1 - f64::from(layout.text.y()),
                            ),
                            false,
                            1,
                        );
                    } else if position.1 < 36.0 && self.pressed.is_none() {
                        return WindowAction::Move;
                    }
                }
            }
            WindowEvent::PointerMove { position } => {
                if let Some(drag) = self.scroll_drag {
                    if let Some(bar) = self.editor.scrollbar(&mut self.fonts, layout.viewport) {
                        self.editor
                            .scroll_to(&mut self.fonts, drag.value(position, &bar));
                    }
                } else if self.dragging {
                    self.editor.pointer(
                        &mut self.fonts,
                        (
                            position.0 - f64::from(layout.text.x()),
                            position.1 - f64::from(layout.text.y()),
                        ),
                        true,
                        1,
                    );
                } else {
                    return WindowAction::None;
                }
            }
            WindowEvent::Scroll { position, delta } if contains(layout.viewport, position) => {
                self.editor.scroll(&mut self.fonts, delta.1 as f32);
            }
            _ => return WindowAction::None,
        }
        WindowAction::Redraw
    }
}
