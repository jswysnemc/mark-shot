//! 【Windows字段测试】【原生路由】同窗口字段切换必须改变原生目标，旧候选不能进入新字段

use super::*;
use std::sync::{Arc, Mutex};

struct Fields {
    target: Arc<Mutex<u64>>,
    commits: Arc<Mutex<Vec<String>>>,
}

impl WindowContent for Fields {
    /// 【Windows字段测试】【显示底色】按 canvas 绘制背景，scale 不影响纯色，无返回值
    fn draw(&mut self, canvas: &mut Pixmap, _scale: f32) {
        canvas.fill(tiny_skia::Color::WHITE);
    }

    /// 【Windows字段测试】【消息记录】按 event 记录实际交付的正文，返回无需操作
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        if let WindowEvent::TextInput(edit) = event {
            self.commits.lock().unwrap().push(edit.commit);
        }
        WindowAction::None
    }

    /// 【Windows字段测试】【当前字段】无参数，返回独立且可切换的字段身份
    fn text_input_target(&self) -> Option<u64> {
        Some(*self.target.lock().unwrap())
    }

    /// 【Windows字段测试】【输入快照】按 focus、target_id 和 cause 返回字段快照，previous 不影响这项身份回归
    fn text_input_snapshot(
        &mut self,
        focus: crate::types::text_input::Focus,
        target_id: u64,
        _previous: Option<&crate::types::text_input::Snapshot>,
        cause: crate::types::text_input::ChangeCause,
    ) -> Option<crate::types::text_input::Snapshot> {
        Some(crate::types::text_input::Snapshot {
            focus_id: focus.id,
            target_id,
            monitor_idx: 0,
            surrounding: None,
            cursor_rect: Some((30, 40, 1, 24)),
            composition: None,
            cause,
            content: crate::types::text_input::ContentType::Multiline,
        })
    }
}

/// 【Windows字段测试】【切换过滤】无参数，在私有运行时切换字段并验证过期候选隔离与新字段交付，无返回值
#[test]
#[ignore = "requires the private Windows test runtime"]
fn field_switch_routes_composition_to_the_current_target() {
    assert_eq!(
        std::env::var("MARK_SHOT_PRIVATE_WINDOWS_TEST").as_deref(),
        Ok("1")
    );
    let target = Arc::new(Mutex::new(17));
    let commits = Arc::new(Mutex::new(Vec::new()));
    let queue = native::EventQueue::default();
    let mut surface = Surface::new(
        WindowSpec {
            title: "Mark Shot field regression".into(),
            app_id: "mark-shot-field-regression".into(),
            size: (320, 180),
            minimum: (100, 100),
        },
        Box::new(Fields {
            target: target.clone(),
            commits: commits.clone(),
        }),
        queue.clone(),
        0,
    )
    .unwrap();
    surface.paint().unwrap();
    native::dispatch(&[], Some(std::time::Duration::ZERO)).unwrap();
    while let Some(event) = queue.pop() {
        surface.event(event).unwrap();
    }
    surface.sync_text().unwrap();
    *target.lock().unwrap() = 18;
    surface.sync_text().unwrap();
    for (target, text) in [(17, "旧字段"), (18, "新字段")] {
        surface
            .event(native::Event::Composition {
                id: surface.window.id(),
                target,
                preedit: String::new(),
                cursor: 0,
                commit: text.into(),
            })
            .unwrap();
    }
    assert_eq!(*commits.lock().unwrap(), ["新字段"]);
}
