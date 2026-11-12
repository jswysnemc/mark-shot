//! 【录制倒计时】【输入处理】取消倒计时或拖动小组件，倒计时结束后自动关闭

use super::Countdown;
use crate::{
    backend::window::{LEFT_BUTTON, WindowAction, WindowContent, WindowEvent},
    keys::Key,
};
use std::time::Instant;

impl WindowContent for Countdown {
    /// 【录制倒计时】【置顶请求】无参数，返回始终置顶，保证倒计时可见
    fn always_on_top(&self) -> bool {
        true
    }

    /// 【录制倒计时】【浮层优先级】无参数，返回高于贴图窗口，避免被已有贴图遮挡
    fn above_pinned_windows(&self) -> bool {
        true
    }

    /// 【录制倒计时】【刷新期限】无参数，返回下一次数字变化或结束的时刻
    fn next_deadline(&self) -> Option<Instant> {
        self.deadline
    }

    /// 【录制倒计时】【绘制委托】按 canvas 与 scale 绘制当前剩余秒数，无返回值
    fn draw(&mut self, canvas: &mut tiny_skia::Pixmap, scale: f32) {
        super::draw::draw(self, canvas, scale);
    }

    /// 【录制倒计时】【事件分派】按 event 处理到期、指针与键盘，返回原生窗口动作
    ///
    /// 只有倒计时自然结束才标记完成；取消按钮、Esc 和窗口管理器关闭都视为取消
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        match event {
            WindowEvent::Wake => {
                let Some(schedule) = self.schedule else {
                    return WindowAction::None;
                };
                let now = Instant::now();
                // 1. 【录制倒计时】【倒数结束】标记完成并关闭，采集在小组件销毁后才开始
                if schedule.expired(now) {
                    self.deadline = None;
                    self.completed.set(true);
                    return WindowAction::Close;
                }
                self.deadline = Some(schedule.next_wake(now));
                WindowAction::Redraw
            }
            WindowEvent::PointerMove { position } => {
                let before = Self::over_cancel(self.pointer);
                self.pointer = position;
                if Self::over_cancel(position) == before {
                    WindowAction::None
                } else {
                    WindowAction::Redraw
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
                let over = Self::over_cancel(position);
                if pressed {
                    self.pressed = over;
                    // 2. 【录制倒计时】【拖动小组件】按钮以外的区域交给窗口管理器移动
                    if !over {
                        return WindowAction::Move;
                    }
                } else if std::mem::take(&mut self.pressed) && over {
                    return WindowAction::Close;
                }
                WindowAction::Redraw
            }
            WindowEvent::Key(input) if input.key == Some(Key::Escape) => WindowAction::Close,
            _ => WindowAction::None,
        }
    }
}
