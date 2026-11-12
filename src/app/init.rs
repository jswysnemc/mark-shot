// Contains all of EditorState initialization logic, including building base pixmaps
// from captured frames, rendering layers, monitor placements, toolbar icon cache,
// and performing the initial paint before entering the main event loop

use crate::backend::ScreenOverlay;
use crate::editor::EditorState;
use crate::profiler::Profiler;
use crate::types::{Capture, MonitorFrame, Output, Placement};
use crate::ui::icons;
use std::collections::HashMap;
use tiny_skia::{IntSize, Pixmap};
use usvg::Tree;

/// 【截图初始化】【冻结缓冲】按 frames 导入原始像素和 Wayland 分数比例，返回截图列表或尺寸错误，不按逻辑尺寸重采样
pub fn build_captures(frames: Vec<MonitorFrame>) -> Result<Vec<Capture>, String> {
    frames
        .into_iter()
        .enumerate()
        .map(|(monitor_idx, mut f)| {
            let (src_w, src_h) = (f.pw_width, f.pw_height);
            let size = IntSize::from_wh(src_w, src_h).ok_or_else(|| {
                format!("Invalid frame size for monitor {monitor_idx}: {src_w}x{src_h}")
            })?;
            let logical_w = f.info.size.map_or(src_w as i32, |s| s.0).max(1);
            let ratio = src_w as f32 / logical_w as f32;
            // 1. 【截图初始化】【分数比例】协议使用一百二十分之一单位，逻辑尺寸取整不能引入额外像素缩放
            let quantized = (ratio * 120.0).round() / 120.0;
            let scale = if quantized > 0.0 { quantized } else { ratio };

            let row_bytes = (src_w as usize) * 4;
            let src_stride = f.pw_stride as usize;
            let tight_len = row_bytes * src_h as usize;

            // for small performance gain giving directly the frame to pixmap without any copying
            if src_stride == row_bytes && f.pixels.len() >= tight_len {
                f.pixels.truncate(tight_len);
                let pixmap = Pixmap::from_vec(f.pixels, size)
                    .ok_or_else(|| format!("Invalid frame for monitor {monitor_idx}"))?;
                return Ok(Capture {
                    pixmap: crate::types::frozen_pixmap::FrozenPixmap::cached(pixmap),
                    scale,
                });
            }

            let mut pixmap = Pixmap::new(src_w, src_h).ok_or_else(|| {
                format!("Invalid frame size for monitor {monitor_idx}: {src_w}x{src_h}")
            })?;
            let dst = pixmap.data_mut();

            if src_stride < row_bytes {
                return Err(format!(
                    "Invalid stride for monitor {}: stride={} row_bytes={}",
                    monitor_idx, src_stride, row_bytes
                ));
            }

            // the last row may come without stride padding
            let needed = src_stride * (src_h as usize - 1) + row_bytes;
            let src = f.pixels.get(..needed).ok_or_else(|| {
                format!(
                    "Not enough pixel data for monitor {}: have={} need={}",
                    monitor_idx,
                    f.pixels.len(),
                    needed
                )
            })?;

            for row in 0..(src_h as usize) {
                let src_off = row * src_stride;
                let dst_off = row * row_bytes;
                dst[dst_off..dst_off + row_bytes]
                    .copy_from_slice(&src[src_off..src_off + row_bytes]);
            }

            Ok(Capture {
                pixmap: crate::types::frozen_pixmap::FrozenPixmap::cached(pixmap),
                scale,
            })
        })
        .collect()
}

/// 【截图初始化】【初始前景】按placements和transparent构造输出图层，返回画布与占位缓存；明确全屏原生前景不写入随后会清除的遮罩
pub(super) fn build_layers_with_transparency(
    placements: &[Placement],
    transparent: bool,
) -> (Vec<Pixmap>, Vec<Pixmap>, Vec<Pixmap>) {
    super::layers::build_with_transparency(placements, transparent)
}

pub fn build_placements(outputs: &[Output], hidpi: bool) -> Vec<Placement> {
    outputs
        .iter()
        .map(|o| Placement {
            position: o.info.logical_position.unwrap_or(o.info.location),
            size: o.info.logical_size.unwrap_or_else(|| {
                o.info
                    .modes
                    .iter()
                    .find(|m| m.current)
                    .map(|m| m.dimensions)
                    .unwrap_or((0, 0))
            }),
            scale: if hidpi { o.scale() } else { 1.0 },
        })
        .collect()
}

pub fn load_icons_cache() -> HashMap<&'static str, Tree> {
    let opt = usvg::Options::default();

    use strum::IntoEnumIterator;
    let all_svgs = crate::tools::Tool::iter()
        .map(|tool| icons::get_svg_for_tool(tool).0)
        .chain(icons::EXTRA_ICONS.iter().copied())
        .chain(crate::ui::toolbar::icons::EXTRA.iter().copied());

    let mut cache = HashMap::new();
    for svg_str in all_svgs {
        cache.entry(svg_str).or_insert_with(|| {
            Tree::from_str(svg_str, &opt).expect("Critical: Failed to parse embedded SVG icon")
        });
    }
    cache
}

pub fn initial_paint(
    editor_state: &mut EditorState,
    overlay: &mut Box<dyn ScreenOverlay>,
    prof: &mut Profiler,
) -> Result<(), Box<dyn std::error::Error>> {
    for i in 0..editor_state.canvas.len() {
        super::frame_backdrop::stage(editor_state, overlay.as_mut(), i, None, true)?;
    }
    overlay.flush()?;
    prof.mark("frames staged + flushed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::StreamInfo;

    // every byte of row padding is 0xEE, so leaked padding is recognisable in the pixmap
    fn frame(w: u32, h: u32, stride: u32, len: usize, logical: Option<(i32, i32)>) -> MonitorFrame {
        let mut pixels = vec![0xEE; len];
        for y in 0..h as usize {
            for x in 0..w as usize {
                let off = y * stride as usize + x * 4;
                if off + 4 <= len {
                    pixels[off..off + 4].copy_from_slice(&[x as u8, y as u8, 0, 255]);
                }
            }
        }

        MonitorFrame {
            output: 0,
            pixels,
            pw_width: w,
            pw_height: h,
            pw_stride: stride,
            info: StreamInfo {
                node_id: 0,
                size: logical,
                position: None,
            },
        }
    }

    fn tight(w: u32, h: u32) -> Vec<u8> {
        (0..h)
            .flat_map(|y| (0..w).flat_map(move |x| [x as u8, y as u8, 0, 255]))
            .collect()
    }

    #[test]
    fn frames_from_every_backend_become_the_same_pixmap() {
        let cases = [
            (
                "kde: always tight, it de-pads KWin's stride itself",
                4,
                3,
                16,
                48,
            ),
            (
                "image-copy: tight, dimensions already un-rotated",
                3,
                4,
                12,
                48,
            ),
            ("portal: padded rows", 4, 3, 32, 96),
            ("portal: padded rows, the last one unpadded", 4, 3, 32, 80),
        ];

        for (label, w, h, stride, len) in cases {
            let logical = Some((w as i32, h as i32));
            let captures = build_captures(vec![frame(w, h, stride, len, logical)])
                .unwrap_or_else(|e| panic!("{label}: {e}"));
            let pixmap = &captures[0].pixmap;

            assert_eq!((pixmap.width(), pixmap.height()), (w, h), "{label}");
            assert_eq!(pixmap.data(), tight(w, h), "{label}");
            assert!(
                !pixmap.data().contains(&0xEE),
                "{label}: row padding leaked into the pixmap"
            );
            assert_eq!(captures[0].scale, 1.0, "{label}");
        }
    }

    #[test]
    fn broken_frames_are_reported_not_panicking() {
        let broken = [
            ("zero width", 0, 3, 0, 0),
            ("zero height", 4, 0, 16, 0),
            ("stride smaller than a row", 4, 3, 12, 36),
            ("tight but truncated", 4, 3, 16, 47),
            ("padded and truncated", 4, 3, 32, 79),
        ];

        for (label, w, h, stride, len) in broken {
            let built = build_captures(vec![frame(w, h, stride, len, None)]);
            assert!(built.is_err(), "{label}: expected an error, not a pixmap");
        }

        // both KWin and PipeWire's chunk.size() may deliver a longer tail
        // funny bug: for some reason, only and only in World of Tanks, kwin was delivering a shorter tail, like -20px
        // which was causing the programm to panick. Honestly, no idea what was that, none of my fixes didn't work out
        // it was expected, since kdescreenshot protocol is doing something wrong, not our end
        // so now if the screenshot has missing pixels, fill them with transparency
        let extra = build_captures(vec![frame(4, 3, 16, 64, None)]);
        assert!(
            extra.is_ok(),
            "trailing bytes past the frame must be ignored, not rejected"
        );
    }

    #[test]
    fn the_scale_comes_from_the_logical_size() {
        let cases = [
            ("hidpi", 3840u32, Some((1920, 1080)), 2.0),
            (
                "fractional logical dimensions are rounded",
                1280,
                Some((853, 533)),
                1.5,
            ),
            ("no logical size at all", 1920, None, 1.0),
        ];

        for (label, w, logical, expected) in cases {
            let captures = build_captures(vec![frame(w, 1, w * 4, (w * 4) as usize, logical)])
                .unwrap_or_else(|e| panic!("{label}: {e}"));
            assert_eq!(captures[0].scale, expected, "{label}");
        }

        for logical in [Some((0, 0)), Some((-1, 1080))] {
            let captures = build_captures(vec![frame(64, 1, 256, 256, logical)]).unwrap();
            assert!(
                captures[0].scale.is_finite(),
                "a {logical:?} logical size must not produce inf"
            );
        }
    }
}
