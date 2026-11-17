//! 【录制控制面板】【隐藏提示】短暂提示截图与托盘入口，关闭桌面通知时仍可了解录制控制方式

use super::{fonts, layout};
use crate::{
    backend::window::{self, LEFT_BUTTON, WindowAction, WindowContent, WindowEvent, WindowSpec},
    keys::Key,
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::controls,
};
use std::time::{Duration, Instant};
use tiny_skia::{Pixmap, Rect};

struct HiddenNotice {
    fonts: fonts::Shared,
    deadline: Option<Instant>,
}

impl WindowContent for HiddenNotice {
    /// 【录制隐藏提示】【圆角外观】无参数，返回启用圆角外侧透明裁切
    fn rounded_corners(&self) -> bool {
        true
    }

    /// 【录制隐藏提示】【置顶请求】无参数，返回置顶显示，提示结束后自动消失
    fn always_on_top(&self) -> bool {
        true
    }

    /// 【录制隐藏提示】【浮层优先级】无参数，返回高于普通贴图，保证控制入口说明可见
    fn above_pinned_windows(&self) -> bool {
        true
    }

    /// 【录制隐藏提示】【采集排除】无参数，返回系统采集排除请求，支持此属性的 Windows 不录入提示
    fn exclude_from_capture(&self) -> bool {
        true
    }

    /// 【录制隐藏提示】【关闭期限】无参数，返回首帧显示后自动关闭的时刻
    fn next_deadline(&self) -> Option<Instant> {
        self.deadline
    }

    /// 【录制隐藏提示】【首帧通知】无参数，从实际首帧开始保留提示四秒，窗口创建耗时不缩短提示
    fn mapped(&mut self) {
        self.deadline = Some(Instant::now() + Duration::from_secs(4));
    }

    /// 【录制隐藏提示】【内容绘制】按 canvas 与 scale 显示录制继续和两个控制入口，复用统一控件，无返回值
    fn draw(&mut self, canvas: &mut Pixmap, scale: f32) {
        let chinese =
            crate::ui::language::Language::current() == crate::ui::language::Language::Chinese;
        let (title, first, second, close) = if chinese {
            (
                "录制控制面板已隐藏",
                "录制继续进行，可再次进入截图界面",
                "或通过托盘暂停、继续和停止录制",
                "知道了",
            )
        } else {
            (
                "Recording controls hidden",
                "Recording continues. Open capture again or",
                "use the tray to pause, resume and stop recording.",
                "OK",
            )
        };
        let palette = Palette::current();
        canvas.fill(palette.window.color());
        let mut shared = self.fonts.borrow_mut();
        let fonts = &mut *shared;
        fonts
            .system
            .ensure_text(&format!("{title}{first}{second}{close}"));
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut fonts.system, &mut fonts.cache);
        painter.text(
            Rect::from_xywh(12.0, 10.0, 278.0, 24.0).unwrap(),
            title,
            TextStyle::new(11.0, palette.text),
        );
        controls::button(
            &mut painter,
            Rect::from_xywh(296.0, 10.0, 52.0, 26.0).unwrap(),
            close,
            false,
            false,
            true,
        );
        for (y, text) in [(43.0, first), (61.0, second)] {
            let text = controls::fit_label(&mut painter, text, 10.0, 336.0);
            painter.text(
                Rect::from_xywh(12.0, y, 336.0, 16.0).unwrap(),
                &text,
                TextStyle::new(10.0, palette.muted),
            );
        }
        crate::ui::window_frame::draw(painter.canvas(), scale);
    }

    /// 【录制隐藏提示】【关闭输入】按 event 在期限、Esc 或鼠标释放时关闭提示，返回动作；不发送任何录制命令
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        let close = match event {
            WindowEvent::Wake => self
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline),
            WindowEvent::Key(input) => input.key == Some(Key::Escape),
            WindowEvent::PointerButton {
                button: LEFT_BUTTON,
                pressed: false,
                ..
            } => true,
            _ => false,
        };
        if close {
            self.deadline = None;
            WindowAction::Close
        } else {
            WindowAction::None
        }
    }
}

/// 【录制隐藏提示】【窗口显示】按 fonts 显示四秒控制入口提示，返回原生窗口结果；提示关闭后录制继续
pub(super) fn run(fonts: fonts::Shared) -> crate::AppResult {
    window::run(
        WindowSpec {
            title: "Mark Shot — Recording controls hidden".into(),
            app_id: "mark-shot-recording-controls-hidden".into(),
            size: layout::SIZE,
            minimum: layout::SIZE,
        },
        HiddenNotice {
            fonts,
            deadline: None,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【录制隐藏提示测试】【显示期限】首帧后安排关闭，期限前不关闭，期限结束清除后续刷新
    #[test]
    fn hidden_notice_is_transient_and_waits_for_first_frame() {
        let mut notice = HiddenNotice {
            fonts: fonts::shared(),
            deadline: None,
        };
        assert!(notice.next_deadline().is_none());
        notice.mapped();
        assert!(notice.next_deadline().unwrap() > Instant::now());
        assert_eq!(notice.event(WindowEvent::Wake), WindowAction::None);
        notice.deadline = Some(Instant::now());
        assert_eq!(notice.event(WindowEvent::Wake), WindowAction::Close);
        assert!(notice.next_deadline().is_none());
    }
}
