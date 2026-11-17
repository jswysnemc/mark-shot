//! 【录制控制面板】【事件处理】隐藏只关闭窗口，录制命令在界面线程以外按顺序交付

use super::{
    POLL, Panel,
    layout::{self, Control},
};
use crate::{
    backend::window::{LEFT_BUTTON, WindowAction, WindowContent, WindowCursor, WindowEvent},
    keys::Key,
};
use std::time::Instant;

impl Panel {
    /// 【录制控制面板】【隐藏请求】无参数，标记需要控制入口提示并关闭面板，返回关闭动作；不发送停止命令
    fn hide(&mut self) -> WindowAction {
        self.hidden.set(true);
        self.deadline = None;
        WindowAction::Close
    }

    /// 【录制控制面板】【按钮执行】按 control 关闭面板或异步发送录制命令，返回窗口动作
    fn activate(&mut self, control: Control) -> WindowAction {
        if control == Control::Hide {
            return self.hide();
        }
        if let Some(command) = self.model.request(control, Instant::now()) {
            self.query.command(command);
            WindowAction::Redraw
        } else {
            WindowAction::None
        }
    }

    /// 【录制控制面板】【状态观察】按 now 领取控制结果并读取本地会话，返回重绘或自动关闭动作
    fn poll(&mut self, now: Instant) -> WindowAction {
        let mut changed = false;
        while let Some(result) = self.query.control_result() {
            let error = match result {
                Err(error) => Some(error),
                Ok(status) if status.detail.contains("Control request failed") => {
                    Some(status.detail)
                }
                _ => None,
            };
            if let Some(error) = error {
                self.model.failed(error);
                changed = true;
            }
        }
        let (active, status) = (self.snapshot)();
        // 1. 【录制控制面板】【会话完成】线程自然结束时关闭窗口，不发送隐藏通知
        if !active {
            self.deadline = None;
            return WindowAction::Close;
        }
        changed |= self.model.update(status, now);
        self.deadline = Some(now + POLL);
        if changed {
            WindowAction::Redraw
        } else {
            WindowAction::None
        }
    }
}

impl WindowContent for Panel {
    /// 【录制控制面板】【圆角外观】无参数，返回启用圆角外侧透明裁切
    fn rounded_corners(&self) -> bool {
        true
    }

    /// 【录制控制面板】【置顶请求】无参数，返回持续置顶，便于录制时操作
    fn always_on_top(&self) -> bool {
        true
    }

    /// 【录制控制面板】【浮层优先级】无参数，返回高于普通贴图，避免控制按钮被贴图遮挡
    fn above_pinned_windows(&self) -> bool {
        true
    }

    /// 【录制控制面板】【采集排除】无参数，返回系统采集排除请求，支持此属性的 Windows 不录入面板
    fn exclude_from_capture(&self) -> bool {
        true
    }

    /// 【录制控制面板】【刷新期限】无参数，返回观察本地会话的下一次期限
    fn next_deadline(&self) -> Option<Instant> {
        self.deadline
    }

    /// 【录制控制面板】【窗口关闭】无参数，窗口管理器关闭也只隐藏面板，返回允许关闭
    fn close_requested(&mut self) -> bool {
        self.hide();
        true
    }

    /// 【录制控制面板】【指针形状】按 position 返回按钮指针或拖动指针
    fn cursor(&self, position: (f64, f64)) -> WindowCursor {
        if layout::hit(position).is_some_and(|control| self.model.enabled(control)) {
            WindowCursor::Pointer
        } else {
            WindowCursor::Grab
        }
    }

    /// 【录制控制面板】【绘制委托】按 canvas 与 scale 绘制当前状态，无返回值
    fn draw(&mut self, canvas: &mut tiny_skia::Pixmap, scale: f32) {
        super::draw::draw(self, canvas, scale);
    }

    /// 【录制控制面板】【输入分派】按 event 处理刷新、点击、拖动与 Esc，返回原生窗口动作
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        match event {
            WindowEvent::Wake => self.poll(Instant::now()),
            WindowEvent::PointerMove { position } => {
                let changed = layout::hit(self.pointer) != layout::hit(position);
                self.pointer = position;
                if changed {
                    WindowAction::Redraw
                } else {
                    WindowAction::None
                }
            }
            WindowEvent::PointerLeave => {
                self.pointer = (-1.0, -1.0);
                WindowAction::Redraw
            }
            WindowEvent::PointerButton {
                position,
                button: LEFT_BUTTON,
                pressed,
            } => {
                let control = layout::hit(position);
                self.pointer = position;
                if pressed {
                    self.pressed = control;
                    if control.is_none() {
                        return WindowAction::Move;
                    }
                } else if let Some(previous) = self.pressed.take()
                    && control == Some(previous)
                {
                    return self.activate(previous);
                }
                WindowAction::Redraw
            }
            WindowEvent::Key(input) if input.key == Some(Key::Escape) => self.hide(),
            WindowEvent::Focused(false) => {
                self.pressed = None;
                WindowAction::None
            }
            _ => WindowAction::None,
        }
    }
}
