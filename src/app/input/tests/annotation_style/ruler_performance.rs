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

/// 【尺子性能测试】【四千像素场景】按scale创建真实3840×2160截图缓冲，返回与逻辑输出尺寸一致的编辑器，不使用缩略图
fn capture_4k(scale: f32) -> EditorState {
    let mut state = capture_editor(scale);
    let logical = ((3840.0 / scale) as i32, (2160.0 / scale) as i32);
    state.placements[0].size = logical;
    let mut image = Pixmap::new(3840, 2160).unwrap();
    image.fill(Color::WHITE);
    state.captures[0].pixmap = image.clone().into();
    state.canvas[0] = image.clone();
    state.dimmed[0] = image;
    state.annotations_layer[0] = Pixmap::new(3840, 2160).unwrap();
    state.selection.zone = Rect::from_xywh(0.0, 0.0, logical.0 as f32, logical.1 as f32);
    state
}

/// 【尺子性能测试】【四千像素生产帧】无参数，测量大范围长度尺与角度尺真实输入及CPU绘制，输出分位数，不代表合成器呈现帧率
#[test]
#[ignore = "requires isolated release timing with one test thread"]
fn measure_4k_ruler_input_and_render_frames() {
    for scale in [1.0, 1.5] {
        for mode in crate::features::ruler::RulerMode::ALL {
            let mut state = capture_4k(scale);
            state.ui_hidden = true;
            let mut dirty = 0;
            state.tool_settings.ruler_mode = mode;
            select_tool(&mut state, Tool::Ruler, &mut dirty);
            pointer(&mut state, 500.0, 400.0, &mut dirty);
            handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
            if mode == crate::features::ruler::RulerMode::Angle {
                pointer(&mut state, 1100.0, 400.0, &mut dirty);
                handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
            }
            super::navigation::render(&mut state, None);
            let mut times = Vec::new();
            let mut damaged_pixels = Vec::new();
            for frame in 0..80 {
                state.damage_rects.clear();
                state.layer_damage_rects.clear();
                state.selection.prev_zone = state.selection.zone;
                dirty = 0;
                let start = Instant::now();
                pointer(
                    &mut state,
                    1500.0 + frame as f64 * 2.0,
                    900.0 + frame as f64 * 0.8,
                    &mut dirty,
                );
                crate::editor::ruler_labels::refresh(&mut state, &mut dirty);
                let area = state.monitor_dirty_rect(0);
                super::navigation::render(&mut state, area);
                if frame >= 20 {
                    times.push(start.elapsed().as_secs_f64() * 1000.0);
                    damaged_pixels.push(area.map_or(3840.0 * 2160.0, |rect| {
                        rect.width() * rect.height() * scale * scale
                    }));
                }
            }
            times.sort_by(f64::total_cmp);
            damaged_pixels.sort_by(f32::total_cmp);
            println!(
                "{{\"mode\":\"{}\",\"physical\":[3840,2160],\"scale\":{scale},\"samples\":{},\"median_ms\":{},\"p95_ms\":{},\"max_ms\":{},\"median_damaged_pixels\":{},\"includes_native_present\":false}}",
                mode.key(),
                times.len(),
                times[30],
                times[57],
                times[59],
                damaged_pixels[30]
            );
        }
    }
}
