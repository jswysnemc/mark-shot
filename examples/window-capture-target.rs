//! 【窗口捕获回归】【确定性窗口】私有合成器中的独立颜色与透明度来源

use mark_shot::backend::window::{self, WindowAction, WindowContent, WindowEvent, WindowSpec};
use tiny_skia::{Pixmap, PremultipliedColorU8};

struct Target {
    identity: u8,
    trace: std::path::PathBuf,
}

impl WindowContent for Target {
    /// 【窗口捕获回归】【原图绘制】按 canvas 的物理尺寸生成四类透明度与身份颜色，scale 写入 trace，无返回值
    fn draw(&mut self, canvas: &mut Pixmap, scale: f32) {
        let (width, height) = (canvas.width(), canvas.height());
        for (index, pixel) in canvas.pixels_mut().iter_mut().enumerate() {
            let (x, y) = (index as u32 % width, index as u32 / width);
            let alpha = match (x >= width / 2, y >= height / 2) {
                (false, false) => 255,
                (true, false) => 128,
                (false, true) => 64,
                (true, true) => 0,
            };
            let color: (u8, u8, u8) = if self.identity == 0 {
                (224, 96, 32)
            } else {
                (32, 160, 224)
            };
            *pixel = PremultipliedColorU8::from_rgba(
                (u16::from(color.0) * u16::from(alpha) / 255) as u8,
                (u16::from(color.1) * u16::from(alpha) / 255) as u8,
                (u16::from(color.2) * u16::from(alpha) / 255) as u8,
                alpha,
            )
            .unwrap();
        }
        // 1. 【窗口捕获回归】【来源记录】先写入临时文件，再原子替换，避免验证读取到截断的半份记录
        let pending = self.trace.with_extension("next");
        std::fs::write(
            &pending,
            serde_json::to_vec(&serde_json::json!({
                "identity": self.identity,
                "width": width,
                "height": height,
                "scale": scale,
            }))
            .unwrap(),
        )
        .unwrap();
        std::fs::rename(pending, &self.trace).unwrap();
    }

    /// 【窗口捕获回归】【窗口输入】忽略 event 中业务操作，由测试运行器关闭本进程；返回无动作
    fn event(&mut self, _event: WindowEvent) -> WindowAction {
        WindowAction::None
    }
}

/// 【窗口捕获回归】【私有入口】从参数读取身份与 trace，在已记录的私有运行目录建立窗口；返回窗口运行错误
fn main() -> mark_shot::AppResult {
    let root = std::path::PathBuf::from(std::env::var("XDG_RUNTIME_DIR")?);
    if !root.join("runtime.json").is_file() {
        return Err("window capture target requires an owned private runtime".into());
    }
    let identity = std::env::args()
        .nth(1)
        .ok_or("identity required")?
        .parse()?;
    let trace = std::env::args_os()
        .nth(2)
        .ok_or("trace path required")?
        .into();
    mark_shot::config::init(None, |_| {});
    window::run(
        WindowSpec {
            title: format!("Window capture target {identity}"),
            app_id: format!("mark-shot-window-capture-target-{identity}"),
            size: (360, 240),
            minimum: (64, 64),
        },
        Target { identity, trace },
    )
}
