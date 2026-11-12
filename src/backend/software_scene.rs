//! 【截图场景】【软件组合】Windows 和 X11 直接合成到持久呈现帧，避免整屏遮罩前景

use crate::types::capture_scene::Frame;
use tiny_skia::IntRect;

mod shade;
mod spans;

/// 【截图场景】【预乘叠加】按 front/back 返回前景覆盖背景的预乘像素，采用现有整数舍入规则
fn over(front: [u8; 4], back: [u8; 4]) -> [u8; 4] {
    if front[3] == 0 {
        return back;
    }
    if front[3] == 255 {
        return front;
    }
    let inverse = 255 - u32::from(front[3]);
    std::array::from_fn(|channel| {
        (u32::from(front[channel]) + (u32::from(back[channel]) * inverse + 127) / 255).min(255)
            as u8
    })
}

/// 【截图场景】【边界转换】按 rect/size 得到与原有遮罩相同的物理范围，空矩形返回空值
fn bounds(rect: Option<tiny_skia::Rect>, size: (u32, u32)) -> Option<IntRect> {
    rect.and_then(|rect| crate::renderer::rect_bounds(&rect, size.0, size.1))
        .and_then(|(x, y, w, h)| IntRect::from_xywh(x as i32, y as i32, w, h))
}

/// 【截图场景】【像素归属】按 rect/x/y 返回整数像素是否位于半开矩形内，空矩形视为外部
fn contains(rect: Option<IntRect>, x: i32, y: i32) -> bool {
    rect.is_some_and(|rect| {
        x >= rect.left() && x < rect.right() && y >= rect.top() && y < rect.bottom()
    })
}

/// 【截图场景】【单次合成】将 frame 合成到 target 的损坏范围，bgra 指定 Windows 通道顺序，返回尺寸错误并保留其他像素
pub(crate) fn compose(frame: &Frame<'_>, target: &mut [u8], bgra: bool) -> crate::AppResult {
    let source = &*frame.foreground;
    let size = (source.width(), source.height());
    if target.len() != source.data().len()
        || size != (frame.background.width(), frame.background.height())
    {
        return Err("software scene pixel grids do not match".into());
    }
    let full = IntRect::from_xywh(0, 0, size.0, size.1).ok_or("empty software scene")?;
    let dirty = match frame.damage {
        Some((x, y, w, h)) => {
            IntRect::from_xywh(x as i32, y as i32, w, h).and_then(|rect| rect.intersect(&full))
        }
        None => Some(full),
    };
    let Some(dirty) = dirty else {
        return Ok(());
    };
    if frame.opaque {
        let stride = size.0 as usize * 4;
        for row in dirty.top()..dirty.bottom() {
            let start = row as usize * stride + dirty.left() as usize * 4;
            let end = start + dirty.width() as usize * 4;
            if bgra {
                crate::utils::copy_swizzled(&mut target[start..end], &source.data()[start..end]);
            } else {
                target[start..end].copy_from_slice(&source.data()[start..end]);
            }
        }
        return Ok(());
    }
    let hole = bounds(frame.selection, size);
    let hover = bounds(frame.hovered, size);
    let pixel = crate::config::get()
        .selection
        .dim
        .color()
        .to_color_u8()
        .premultiply();
    let dim = [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()];
    let pixel = tiny_skia::ColorU8::from_rgba(94, 234, 212, 32).premultiply();
    let tint = [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()];
    let stride = size.0 as usize * 4;
    // 1. 【截图场景】【逐行访问】原图可能来自映射文件，使用有界访问保留原缓存回收策略
    frame
        .background
        .visit_rows_with_budget(dirty, 2 * 1024 * 1024, |y, background| {
            let start = y as usize * stride + dirty.left() as usize * 4;
            spans::row(
                &source.data()[start..start + background.len()],
                background,
                &mut target[start..start + background.len()],
                dirty,
                y as i32,
                hole,
                hover,
                dim,
                tint,
                bgra,
            );
        });
    Ok(())
}

#[cfg(test)]
#[path = "software_scene_tests.rs"]
mod tests;
