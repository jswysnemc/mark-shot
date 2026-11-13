//! 【滚动截图回归】【原生页面】为私有合成器提供可追溯滚轮与键盘输入的确定性页面

use mark_shot::{
    backend::window::{self, WindowAction, WindowContent, WindowEvent, WindowSpec},
    keys::Key,
};
use tiny_skia::{Pixmap, PremultipliedColorU8};

struct Page {
    origin: (i32, i32),
    trace: std::path::PathBuf,
}

impl Page {
    /// 【滚动截图回归】【页面移动】按 dx、dy 更新原图位置并记录 trace，返回重绘请求
    fn advance(&mut self, dx: i32, dy: i32) -> WindowAction {
        self.origin.0 = (self.origin.0 + dx).max(0);
        self.origin.1 = (self.origin.1 + dy).max(0);
        std::fs::write(
            &self.trace,
            format!("{} {}\n", self.origin.0, self.origin.1),
        )
        .unwrap();
        WindowAction::Redraw
    }
}

impl WindowContent for Page {
    /// 【滚动截图回归】【页面绘制】按 canvas 与 scale 绘制固定文档坐标的确定性纹理，无返回值
    fn draw(&mut self, canvas: &mut Pixmap, scale: f32) {
        let width = canvas.width();
        for (index, pixel) in canvas.pixels_mut().iter_mut().enumerate() {
            let x = (index as u32 % width) as i32 + (self.origin.0 as f32 * scale).round() as i32;
            let y = (index as u32 / width) as i32 + (self.origin.1 as f32 * scale).round() as i32;
            let hash = (y as u32).wrapping_mul(2_654_435_761).rotate_left(7)
                ^ (x as u32).wrapping_mul(2_246_822_519);
            *pixel = PremultipliedColorU8::from_rgba(
                (hash & 255) as u8,
                ((hash >> 8) & 255) as u8,
                ((hash >> 16) & 255) as u8,
                255,
            )
            .unwrap();
        }
    }

    /// 【滚动截图回归】【页面输入】按 event 接收底层真实滚轮和方向键，返回窗口动作
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        match event {
            WindowEvent::Scroll { delta, .. } => self.advance(
                if delta.0 == 0.0 {
                    0
                } else {
                    (delta.0.signum() * 48.0) as i32
                },
                if delta.1 == 0.0 {
                    0
                } else {
                    (delta.1.signum() * 48.0) as i32
                },
            ),
            WindowEvent::Key(input) => match input.key {
                Some(Key::Down) => self.advance(0, 48),
                Some(Key::Right) => self.advance(48, 0),
                Some(Key::Home) => {
                    self.origin = (0, 0);
                    self.advance(0, 0)
                }
                Some(Key::Escape) => WindowAction::Close,
                _ => WindowAction::None,
            },
            _ => WindowAction::None,
        }
    }
}

/// 【滚动截图回归】【测试入口】用命令参数指定本次 trace，仅在有私有会话标记时建立页面，无返回值
fn main() -> mark_shot::AppResult {
    // 1. 【滚动截图回归】【隔离校验】各平台只接受自己的私有运行时标记，避免在用户桌面创建测试页面
    #[cfg(target_os = "linux")]
    {
        let root = std::path::PathBuf::from(std::env::var("XDG_RUNTIME_DIR")?);
        if !root.join("runtime.json").is_file() {
            return Err("scroll page requires an owned private runtime".into());
        }
    }
    #[cfg(target_os = "windows")]
    if std::env::var("MARK_SHOT_PRIVATE_WINDOWS_TEST").as_deref() != Ok("1") {
        return Err("scroll page requires the private Windows test runtime".into());
    }
    let trace = std::env::args_os()
        .nth(1)
        .ok_or("trace path required")?
        .into();
    mark_shot::config::init(None, |_| {});
    window::run(
        WindowSpec {
            title: "Scroll test page".into(),
            app_id: "mark-shot-scroll-test-page".into(),
            size: (1280, 800),
            minimum: (64, 64),
        },
        Page {
            origin: (0, 0),
            trace,
        },
    )
}
