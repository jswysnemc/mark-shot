//! 【录制设置】【输入处理】点击与滚轮更新选项，确认开始后关闭窗口并把请求移交流程

use super::{
    Setup,
    layout::{self, Control},
    options::{self, Outcome, Row},
};
use crate::{
    backend::window::{LEFT_BUTTON, WindowAction, WindowContent, WindowEvent},
    keys::Key,
};

impl Setup {
    /// 【录制设置】【控件动作】按 control 更新选项或提交开始请求，返回窗口动作
    ///
    /// 选项只在开始录制、重新框选或打开设置窗口时写入配置，直接关闭窗口不保存
    fn activate(&mut self, control: Control) -> WindowAction {
        let mut close = false;
        let result: crate::AppResult<()> = (|| {
            match control {
                Control::Close => close = true,
                Control::Field(row) if options::enabled(row, &self.config) => {
                    self.choice = Some((
                        row,
                        crate::ui::controls::Choice::new(
                            layout::field(self.size, layout::index(row)),
                            options::choices(row, &self.config, &self.target, self.chinese()),
                            options::selected(row, &self.config, &self.target),
                        ),
                    ));
                }
                Control::Field(_) => {}
                Control::Choice(row, index) if options::enabled(row, &self.config) => {
                    match options::choose(row, index, &mut self.config, &mut self.target) {
                        Outcome::Redraw => {}
                        // 1. 【录制设置】【重新框选】当前窗口先交出目标与配置，选区完成后由新进程再次打开本窗口
                        Outcome::SelectRegion => {
                            self.save()?;
                            self.host.borrow_mut().set_target(self.target.clone());
                            let target = serde_json::to_string(&self.target)?;
                            crate::utils::spawn_self(
                                &["--record-select", "--record-target", &target],
                                &[],
                            )?;
                            close = true;
                        }
                    }
                }
                Control::Choice(..) => {}
                Control::Cursor => self.config.cursor = !self.config.cursor,
                Control::Advanced => {
                    self.advanced = !self.advanced;
                    self.notice_scroll = 0;
                    self.requested_size = Some((self.size.0, layout::height(self.advanced)));
                }
                Control::Settings => {
                    self.save()?;
                    crate::features::preferences::spawn()?;
                    self.settings_opened = true;
                }
                Control::Copy => {
                    if let Some(failure) = &self.failure {
                        crate::utils::copy_to_clipboard(failure)?;
                    }
                }
                // 2. 【录制设置】【确认开始】先校验并保存选项，再把请求交给流程，窗口关闭后才进入倒计时或采集
                Control::Start => {
                    self.config.validate()?;
                    self.save()?;
                    let mut host = self.host.borrow_mut();
                    host.set_target(self.target.clone());
                    host.request(self.config.clone());
                    close = true;
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.failure = Some(error.to_string());
            self.notice_scroll = 0;
        }
        if close {
            WindowAction::Close
        } else {
            WindowAction::Redraw
        }
    }
}

impl WindowContent for Setup {
    /// 【录制设置】【按需尺寸】无参数，仅在展开或收起高级选项后请求新高度，不覆盖用户平时调整的宽度
    fn preferred_size(&self) -> Option<(u32, u32)> {
        self.requested_size
    }

    /// 【录制设置】【绘制委托】按 canvas 与 scale 绘制当前选项，无返回值
    fn draw(&mut self, canvas: &mut tiny_skia::Pixmap, scale: f32) {
        super::draw::draw(self, canvas, scale);
    }

    /// 【录制设置】【事件分派】按 event 处理指针、滚轮与键盘，返回原生窗口动作
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        if let Some((row, choice)) = &mut self.choice {
            let row = *row;
            let action = choice.event(&event, self.size);
            match action {
                crate::ui::controls::ChoiceEvent::Selected(index) => {
                    self.choice = None;
                    return self.activate(Control::Choice(row, index));
                }
                crate::ui::controls::ChoiceEvent::Closed => self.choice = None,
                crate::ui::controls::ChoiceEvent::Keep => return WindowAction::None,
                crate::ui::controls::ChoiceEvent::Redraw => return WindowAction::Redraw,
            }
            if !matches!(event, WindowEvent::Resize { .. } | WindowEvent::Focused(_)) {
                return WindowAction::Redraw;
            }
        }
        match event {
            WindowEvent::Resize { size } => {
                self.size = size;
                if self.requested_size == Some(size) {
                    self.requested_size = None;
                }
                // 1. 【录制设置】【窄窗收起】用户或合成器缩小高度后保留参数并收起高级区，避免与底部开始按钮重叠
                if self.advanced && size.1 < layout::height(true) {
                    self.advanced = false;
                    self.notice_scroll = 0;
                    self.requested_size = None;
                }
                WindowAction::Redraw
            }
            // 1. 【录制设置】【设置窗口返回】重新读取偏好，本次指定的格式继续覆盖用户默认值
            WindowEvent::Focused(true) if self.settings_opened => {
                self.settings_opened = false;
                if crate::config::preferences::path()
                    .is_ok_and(|path| crate::config::preferences::reload(&path).is_ok())
                {
                    self.reload();
                }
                WindowAction::Redraw
            }
            WindowEvent::PointerMove { position } => {
                // 2. 【录制设置】【悬停重绘】悬停目标未变化时不重绘，避免指针移动反复折行错误详情
                let before = layout::hit(self, self.pointer);
                self.pointer = position;
                if layout::hit(self, position) == before {
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
                let hit = layout::hit(self, position);
                if pressed {
                    self.pointer = position;
                    self.pressed = hit;
                    // 3. 【录制设置】【拖动窗口】标题栏空白处按下交给窗口管理器移动
                    if hit.is_none() && position.1 < f64::from(layout::TITLE) {
                        return WindowAction::Move;
                    }
                } else if self.pressed.take() == hit
                    && let Some(control) = hit
                {
                    return self.activate(control);
                }
                WindowAction::Redraw
            }
            WindowEvent::Scroll { position, delta } => {
                // 4. 【录制设置】【滚轮微调】帧率行按滚轮步进，提示区滚动查看完整失败详情
                if self.advanced
                    && layout::contains(layout::row(self.size, Row::Fps), position)
                    && delta.1 != 0.0
                {
                    options::step_frame_rate(if delta.1 < 0.0 { 1 } else { -1 }, &mut self.config);
                    return WindowAction::Redraw;
                }
                if layout::contains(layout::notice(self.size, self.advanced), position)
                    && delta.1 != 0.0
                {
                    self.notice_scroll = self
                        .notice_scroll
                        .saturating_add_signed(if delta.1 > 0.0 { 1 } else { -1 });
                }
                WindowAction::Redraw
            }
            WindowEvent::Key(input) if input.key == Some(Key::Enter) => {
                self.activate(Control::Start)
            }
            WindowEvent::Key(input) if input.key == Some(Key::Escape) => {
                self.activate(Control::Close)
            }
            _ => WindowAction::None,
        }
    }
}
