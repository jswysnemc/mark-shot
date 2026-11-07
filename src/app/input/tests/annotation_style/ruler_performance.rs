//! 【尺子性能测试】【完整输入帧】测量实际指针、草稿、说明、损坏范围及生产渲染，不以单项文字耗时代表应用帧率

use super::*;
use std::time::Instant;

/// 【尺子性能测试】【连续生产帧】长度和角度草稿在两种设备比例下测量完整帧，保留真实悬停十字线，无参数、无返回值
#[test]
#[ignore = "requires isolated release timing with one test thread"]
fn measure_complete_ruler_drawing_input_and_render_frames() {
    for scale in [1.0, 1.5] {
        for mode in crate::features::ruler::RulerMode::ALL {
            let mut state = capture_editor(scale);
            state.ui_hidden = true;
            let mut dirty = 0;
            state.tool_settings.ruler_mode = mode;
            select_tool(&mut state, Tool::Ruler, &mut dirty);
            pointer(&mut state, 100.0, 100.0, &mut dirty);
            handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
            if mode == crate::features::ruler::RulerMode::Angle {
                pointer(&mut state, 220.0, 100.0, &mut dirty);
                handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
            }
            super::navigation::render(&mut state, None);
            let mut times = Vec::new();
            // 1. 【尺子性能测试】【连续输入】沿同一路径实际更新草稿和悬停，每帧使用生产损坏范围绘制
            for frame in 0..80 {
                state.damage_rects.clear();
                state.layer_damage_rects.clear();
                state.selection.prev_zone = state.selection.zone;
                dirty = 0;
                let start = Instant::now();
                pointer(
                    &mut state,
                    250.0 + frame as f64 * 2.0,
                    190.0 + frame as f64 * 0.8,
                    &mut dirty,
                );
                crate::editor::ruler_labels::refresh(&mut state, &mut dirty);
                let area = state.monitor_dirty_rect(0);
                super::navigation::render(&mut state, area);
                if frame >= 20 {
                    times.push(start.elapsed().as_secs_f64() * 1000.0);
                }
            }
            times.sort_by(f64::total_cmp);
            println!(
                "{{\"mode\":\"{}\",\"scale\":{scale},\"median_ms\":{},\"p95_ms\":{}}}",
                mode.key(),
                times[30],
                times[57]
            );
        }
    }
}
