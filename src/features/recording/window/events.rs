//! 【录制窗口】【输入处理】控制消息与配置编辑分离，关闭活动窗口先完成视频收尾

use super::{
    Recorder,
    layout::{self, Control},
};
use crate::{
    backend::window::{LEFT_BUTTON, WindowAction, WindowContent, WindowEvent},
    features::recording::{Command, Status, Target, controller::Controller},
    keys::Key,
};

impl Recorder {
    /// 【录制窗口】【控件动作】按 control 更新配置或投递录制命令，返回窗口动作
    fn activate(&mut self, control: Control) -> WindowAction {
        let result: crate::AppResult = (|| {
            if self.active()
                && !matches!(
                    control,
                    Control::Close | Control::Copy | Control::Pause | Control::Stop
                )
            {
                return Ok(());
            }
            match control {
                Control::Close => {
                    if self.close_requested() {
                        return Ok(());
                    }
                }
                Control::Copy => crate::utils::copy_to_clipboard(&format!(
                    "{}\n{}",
                    self.status.phase, self.status.detail
                ))?,
                Control::Pause => {
                    if let Some(controller) = &self.controller {
                        controller.send(if self.status.phase == "paused" {
                            Command::Resume
                        } else {
                            Command::Pause
                        })?;
                    }
                }
                Control::Stop => {
                    if self.stop_requested || self.status.phase == "saving" {
                        return Ok(());
                    }
                    if let Some(controller) = &self.controller {
                        controller.send(Command::Stop)?;
                        self.stop_requested = true;
                    }
                }
                Control::Start => {
                    self.save()?;
                    let controller = Controller::start(self.config.clone(), self.target.clone())?;
                    self.server.attach(&controller);
                    self.status = controller.snapshot();
                    self.controller = Some(controller);
                    self.stop_requested = false;
                    self.deadline =
                        Some(std::time::Instant::now() + std::time::Duration::from_millis(250));
                    self.error_scroll = 0;
                }
                Control::Region => {
                    self.save()?;
                    crate::utils::spawn_self(&["--record-select"], &[])?;
                    self.close_after_stop = true;
                }
                Control::Display => {
                    self.target = Target::default();
                }
                Control::Format(index) => self.config.format = ["mp4", "mkv", "gif"][index].into(),
                Control::Audio(index) => {
                    self.config.audio = ["off", "system", "microphone"][index].into()
                }
                Control::Quality(index) => {
                    self.config.quality = ["balanced", "high", "efficient"][index].into()
                }
                Control::Fps(delta) => {
                    let fps = if self.config.format == "gif" {
                        &mut self.config.gif_fps
                    } else {
                        &mut self.config.fps
                    };
                    *fps = (*fps as i32 + delta)
                        .clamp(1, if self.config.format == "gif" { 30 } else { 60 })
                        as u32;
                }
                Control::Countdown => {
                    self.config.countdown = match self.config.countdown {
                        0 => 3,
                        3 => 5,
                        _ => 0,
                    }
                }
                Control::Cursor => self.config.cursor = !self.config.cursor,
                Control::Settings => {
                    self.save()?;
                    crate::features::preferences::spawn()?;
                    self.settings_opened = true;
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.status = Status {
                phase: "failed".into(),
                detail: error.to_string(),
                ..Default::default()
            };
        }
        if (control == Control::Close || self.close_after_stop) && !self.active() {
            WindowAction::Close
        } else {
            WindowAction::Redraw
        }
    }
}

impl WindowContent for Recorder {
    /// 【录制窗口】【关闭收尾】无参数，活动录制先投递停止，结束后允许关闭
    fn close_requested(&mut self) -> bool {
        if self.active() {
            self.close_after_stop = true;
            // 1. 【录制窗口】【等待保存】关闭时已进入导出则等待完成，避免第二次停止取消转换
            if !self.stop_requested
                && let Some(controller) = &self.controller
                && controller.snapshot().phase != "saving"
            {
                self.stop_requested = controller.send(Command::Stop).is_ok();
            }
            false
        } else {
            true
        }
    }

    /// 【录制窗口】【状态刷新】无参数，活动时每 250ms 更新时长，空闲时不定时唤醒
    fn next_deadline(&self) -> Option<std::time::Instant> {
        self.deadline
    }

    /// 【录制窗口】【绘制委托】按 canvas 与 scale 绘制当前轻量状态，无返回值
    fn draw(&mut self, canvas: &mut tiny_skia::Pixmap, scale: f32) {
        super::draw::draw(self, canvas, scale);
    }

    /// 【录制窗口】【事件分派】按 event 处理键盘、指针和后台状态，返回原生窗口动作
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        match event {
            WindowEvent::Resize { size } => self.size = size,
            WindowEvent::Wake => {
                if let Some(controller) = &self.controller {
                    self.status = controller.snapshot();
                }
                self.deadline = self
                    .active()
                    .then(|| std::time::Instant::now() + std::time::Duration::from_millis(250));
                if self.close_after_stop && !self.active() {
                    return WindowAction::Close;
                }
            }
            WindowEvent::Focused(true) if self.settings_opened && !self.active() => {
                self.settings_opened = false;
                if let Ok(path) = crate::config::preferences::path()
                    && crate::config::preferences::reload(&path).is_ok()
                {
                    self.config = crate::config::preferences::current().recording.clone();
                }
            }
            WindowEvent::PointerMove { position } => self.pointer = position,
            WindowEvent::PointerButton {
                position,
                button: LEFT_BUTTON,
                pressed,
            } => {
                let hit = layout::hit(self, position);
                if pressed {
                    self.pressed = hit;
                    if hit.is_none() && position.1 < 44.0 {
                        return WindowAction::Move;
                    }
                } else if self.pressed.take() == hit
                    && let Some(control) = hit
                {
                    return self.activate(control);
                }
            }
            WindowEvent::Scroll { position, delta } => {
                if position.1 >= 202.0
                    && position.1 < 232.0
                    && position.0 >= 120.0
                    && position.0 < 242.0
                    && delta.1 != 0.0
                {
                    return self.activate(Control::Fps(if delta.1 < 0.0 { 1 } else { -1 }));
                }
                if position.1 >= 280.0 {
                    self.error_scroll =
                        self.error_scroll
                            .saturating_add_signed(if delta.1 > 0.0 { 1 } else { -1 });
                }
            }
            WindowEvent::Key(input) if input.key == Some(Key::Escape) => {
                return self.activate(Control::Close);
            }
            _ => return WindowAction::None,
        }
        WindowAction::Redraw
    }
}
