//! 【区域识别原生回归】【确定性面板】只在私有合成器绘制嵌套灰色控件

use mark_shot::backend::window::{self, WindowAction, WindowContent, WindowEvent, WindowSpec};
use tiny_skia::{Pixmap, PremultipliedColorU8};

struct Target {
    trace: std::path::PathBuf,
}

impl WindowContent for Target {
    /// 【区域识别原生回归】【面板绘制】按 canvas 物理尺寸与 scale 绘制外层面板和中心控件，trace 记录实际边界，无返回值
    fn draw(&mut self, canvas: &mut Pixmap, scale: f32) {
        let width = canvas.width() as i32;
        let height = canvas.height() as i32;
        let px = |value: f32| (value * scale).round() as i32;
        let parent = [px(40.0), px(30.0), width - px(80.0), height - px(60.0)];
        let child = [
            (width - px(120.0)) / 2,
            (height - px(80.0)) / 2,
            px(120.0),
            px(80.0),
        ];
        let contains = |rect: [i32; 4], x: i32, y: i32| {
            x >= rect[0] && y >= rect[1] && x < rect[0] + rect[2] && y < rect[1] + rect[3]
        };
        for (index, pixel) in canvas.pixels_mut().iter_mut().enumerate() {
            let (x, y) = (index as i32 % width, index as i32 / width);
            let value = if contains(child, x, y) {
                75
            } else if contains(parent, x, y) {
                45
            } else {
                20
            };
            *pixel = PremultipliedColorU8::from_rgba(value, value, value, 255).unwrap();
        }
        // 1. 【区域识别原生回归】【来源记录】原子更新物理边界，验证器不根据窗口初始尺寸猜测
        let next = self.trace.with_extension("next");
        std::fs::write(
            &next,
            serde_json::to_vec(&serde_json::json!({
                "width":width, "height":height, "scale":scale, "parent":parent, "child":child,
            }))
            .unwrap(),
        )
        .unwrap();
        std::fs::rename(next, &self.trace).unwrap();
    }

    /// 【区域识别原生回归】【窗口事件】忽略 event 的业务操作，验证器独立结束来源进程，返回无动作
    fn event(&mut self, _event: WindowEvent) -> WindowAction {
        WindowAction::None
    }
}

/// 【区域识别原生回归】【隔离入口】按参数读取 trace 并核对私有运行目录，返回窗口运行结果
fn main() -> mark_shot::AppResult {
    let runtime = std::path::PathBuf::from(std::env::var("XDG_RUNTIME_DIR")?);
    if !runtime.join("runtime.json").is_file() {
        return Err("region detection target requires an owned private runtime".into());
    }
    let trace = std::env::args_os()
        .nth(1)
        .ok_or("trace path required")?
        .into();
    mark_shot::config::init(None, |_| {});
    window::run(
        WindowSpec {
            title: "Region detection target".into(),
            app_id: "mark-shot-region-detection-target".into(),
            size: (420, 320),
            minimum: (360, 240),
        },
        Target { trace },
    )
}
