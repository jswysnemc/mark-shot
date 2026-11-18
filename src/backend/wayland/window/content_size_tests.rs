//! 【模态窗口测试】【真实尺寸】私有合成器验证展开和收起时调整外层窗口，不能只检查内容返回的尺寸

use crate::backend::window::{WindowAction, WindowContent, WindowDialog, WindowEvent, WindowSpec};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tiny_skia::Pixmap;

type Frames = Arc<Mutex<Vec<(usize, (u32, u32))>>>;

struct Child {
    stage: usize,
    deadline: Option<Instant>,
    frames: Frames,
}

impl Child {
    /// 【模态窗口测试】【阶段尺寸】无参数，返回当前展开或收起阶段的目标窗口尺寸
    fn size(&self) -> (u32, u32) {
        (344, if self.stage == 1 { 600 } else { 224 })
    }
}

impl WindowContent for Child {
    /// 【模态窗口测试】【尺寸请求】无参数，返回当前阶段需要的完整窗口尺寸
    fn preferred_size(&self) -> Option<(u32, u32)> {
        Some(self.size())
    }

    /// 【模态窗口测试】【阶段期限】无参数，返回首次呈现后启动的阶段切换期限
    fn next_deadline(&self) -> Option<Instant> {
        self.deadline
    }

    /// 【模态窗口测试】【首次呈现】无参数，首帧完成后安排展开，避免启动时间影响初始帧，无返回值
    fn mapped(&mut self) {
        self.deadline = Some(Instant::now() + Duration::from_millis(100));
    }

    /// 【模态窗口测试】【实际画布】按 canvas 和 scale 记录实际逻辑尺寸，frames 保存每一阶段，无返回值
    fn draw(&mut self, canvas: &mut Pixmap, scale: f32) {
        let size = (
            (canvas.width() as f32 / scale).round() as u32,
            (canvas.height() as f32 / scale).round() as u32,
        );
        self.frames.lock().unwrap().push((self.stage, size));
        canvas.fill(tiny_skia::Color::WHITE);
    }

    /// 【模态窗口测试】【展开收起】按 event 依次展开、收起和关闭，返回必要窗口动作
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        if !matches!(event, WindowEvent::Wake) {
            return WindowAction::None;
        }
        self.stage += 1;
        self.deadline = Some(Instant::now() + Duration::from_millis(100));
        if self.stage == 3 {
            WindowAction::Close
        } else {
            WindowAction::Redraw
        }
    }
}

struct Parent {
    child: Option<Child>,
}

impl WindowContent for Parent {
    /// 【模态窗口测试】【父窗口绘制】按 canvas 填充父窗口，scale 不改变纯色，无返回值
    fn draw(&mut self, canvas: &mut Pixmap, _scale: f32) {
        canvas.fill(tiny_skia::Color::WHITE);
    }

    /// 【模态窗口测试】【父窗口输入】按 event 接收生命周期事件，返回无需操作
    fn event(&mut self, _event: WindowEvent) -> WindowAction {
        WindowAction::None
    }

    /// 【模态窗口测试】【创建子窗口】无参数，只返回一次真实模态子窗口请求
    fn take_dialog(&mut self) -> Option<WindowDialog> {
        self.child.take().map(|child| WindowDialog {
            id: 1,
            spec: spec(child.size()),
            content: Box::new(child),
        })
    }

    /// 【模态窗口测试】【关闭交接】按 id 和 accepted 接收子窗口结束，返回关闭父窗口
    fn dialog_closed(&mut self, _id: u64, _accepted: bool) -> WindowAction {
        WindowAction::Close
    }
}

/// 【模态窗口测试】【窗口规格】按 size 返回私有窗口尺寸和最小尺寸，不连接实际桌面
fn spec(size: (u32, u32)) -> WindowSpec {
    WindowSpec {
        title: "Mark Shot size regression".into(),
        app_id: "mark-shot-size-regression".into(),
        size,
        minimum: (100, 100),
    }
}

/// 【模态窗口测试】【真实展开收起】无参数，在明确启用的私有 Wayland 会话核对三阶段实际画布尺寸，无返回值
#[test]
#[ignore = "requires a private Wayland compositor"]
fn modal_content_resizes_the_native_surface_on_expansion_and_collapse() {
    assert_eq!(
        std::env::var("MARK_SHOT_PRIVATE_WAYLAND_TEST").as_deref(),
        Ok("1")
    );
    let frames = Frames::default();
    let parent = Parent {
        child: Some(Child {
            stage: 0,
            deadline: None,
            frames: frames.clone(),
        }),
    };
    super::super::runtime::run(spec((360, 320)), parent).unwrap();
    let frames = frames.lock().unwrap();
    for (stage, expected) in [(0, (344, 224)), (1, (344, 600)), (2, (344, 224))] {
        assert!(
            frames.contains(&(stage, expected)),
            "stage {stage} did not draw its native size: {frames:?}"
        );
    }
}
