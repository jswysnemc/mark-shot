//! 【聚焦诊断】【真实拖动】测量真实指针入口与帧绘制，手动运行以避免机器速度影响常规回归

use crate::{
    app::region_rendering_test_support as support,
    features::filter::{FilterMode, RegionFilter},
    features::{annotation_selection::frame, annotation_style::window::WindowUnits},
    tools::Tool,
    types::{AnnotationShape, MouseButton},
};
use std::time::{Duration, Instant};
use tiny_skia::Rect;

/// 【聚焦诊断】【交互测量】按 size、focus 和 resize 执行六次真实指针移动，返回输入与帧绘制平均耗时
fn measure(size: (i32, i32), focus: bool, resize: bool) -> (Duration, Duration) {
    let mut state = support::editor_size(size, 1.0, true);
    let bounds = Rect::from_xywh(0.0, 0.0, size.0 as f32, size.1 as f32).unwrap();
    state.selection.zone = Some(bounds);
    state.ui_hidden = true;
    let large = std::env::var_os("MARK_SHOT_FOCUS_BENCH_LARGE").is_some();
    let end = if large {
        (size.0 as f32 - 400.0, size.1 as f32 - 320.0)
    } else {
        (520.0, 400.0)
    };
    let mut annotation = support::annotation(
        1,
        0.0,
        if focus {
            AnnotationShape::Redact {
                filter: RegionFilter::new((200.0, 160.0), end, 14.0, FilterMode::Focus).unwrap(),
            }
        } else {
            AnnotationShape::Rectangle {
                start: (200.0, 160.0),
                end,
                radius: 12.0,
                filled: false,
                fixed_ratio: false,
            }
        },
    );
    annotation.frozen_bounds = Some(bounds);
    let control = if let AnnotationShape::Redact { filter } = annotation.shape {
        crate::features::filter::drag::frame(filter).unwrap()
    } else {
        frame::bounds(&annotation, WindowUnits::new(1.0)).unwrap()
    };
    let point = if resize {
        (
            f64::from(control.right()),
            f64::from((control.top() + control.bottom()) / 2.0),
        )
    } else {
        (
            f64::from((control.left() + control.right()) / 2.0),
            f64::from((control.top() + control.bottom()) / 2.0),
        )
    };
    state.annotations.push(annotation);
    state.selected_tool = Tool::Pick;
    state.select_annotation_ids([1]);
    support::paint(&mut state, None, false);
    support::finish_frame(&mut state);
    support::pointer(&mut state, point, Some((MouseButton::Left, true)), &mut 0);
    assert!(
        state.ann_drag.is_some(),
        "actual annotation drag must begin"
    );
    support::paint_current(&mut state, None, false);
    support::finish_frame(&mut state);
    let (mut input, mut paint) = (Duration::ZERO, Duration::ZERO);
    // 1. 【聚焦诊断】【实际移动】每帧改变鼠标位置，避免仅测量静止缓存
    for index in 1..=6 {
        let started = Instant::now();
        support::pointer(
            &mut state,
            (
                point.0 + f64::from(index) * 5.0,
                point.1 + f64::from(index) * 2.0,
            ),
            None,
            &mut 0,
        );
        input += started.elapsed();
        let dirty = state.monitor_dirty_rect(0);
        let started = Instant::now();
        support::paint_current(&mut state, dirty, false);
        paint += started.elapsed();
        support::finish_frame(&mut state);
    }
    (input / 6, paint / 6)
}

/// 【聚焦诊断】【帧预算】无参数，对照普通形状与聚焦移动和调整；基础预算三十三毫秒，慢机器或大范围允许普通形状基线加有限开销
#[test]
#[ignore = "手动性能测量，依赖机器速度"]
fn focus_interaction_frame_budget() {
    for size in [(1920, 1080), (3840, 2160)] {
        for resize in [false, true] {
            let mut baseline = Duration::ZERO;
            for focus in [false, true] {
                let (input, paint) = measure(size, focus, resize);
                println!(
                    "【聚焦诊断】【帧耗时】size={size:?} focus={focus} resize={resize} input_ms={:.3} frame_ms={:.3}",
                    input.as_secs_f64() * 1000.0,
                    paint.as_secs_f64() * 1000.0
                );
                if focus {
                    let budget = Duration::from_millis(33)
                        .max(baseline.mul_f64(1.35) + Duration::from_millis(5));
                    assert!(
                        input + paint < budget,
                        "focus movement exceeds frame budget: focus={:?} ordinary={baseline:?} budget={budget:?}",
                        input + paint
                    );
                } else {
                    baseline = input + paint;
                }
            }
        }
    }
}
