//! 【显示画布】【主题底色】仅用于原图之外和透明像素背后的显示，不参与原件或导出

use super::{Mode, Rgba};

/// 【显示画布】【模式配色】接收 mode 并返回与 Qt 一致的棋盘格首色；图片背景独立于控件主题
pub fn background(_mode: Mode) -> Rgba {
    Rgba(51, 55, 62, 255)
}

/// 【显示画布】【当前底色】返回当前主题的显示底色，不读取或修改图像内容
pub fn current() -> Rgba {
    background(Mode::current())
}

/// 【显示画布】【透明底纹】按 scale 将十二逻辑像素棋盘格绘入 canvas；原点固定在窗口，缩放图片时不移动底纹，无返回值
pub fn checkerboard(canvas: &mut tiny_skia::Pixmap, scale: f32) {
    checkerboard_region(canvas, (scale, scale), (0, 0));
}

/// 【显示画布】【局部底纹】按 scale 和 offset 的窗口物理坐标填充 canvas，局部恢复保持与整窗相同相位，无返回值
pub fn checkerboard_region(canvas: &mut tiny_skia::Pixmap, scale: (f32, f32), offset: (i32, i32)) {
    let area = tiny_skia::IntRect::from_xywh(0, 0, canvas.width(), canvas.height()).unwrap();
    checkerboard_area(canvas, scale, offset, area);
}

/// 【显示画布】【范围底纹】按 scale、offset 和 area 恢复 canvas 的相交底纹，返回值为空，窗口外相位保持不变
pub(crate) fn checkerboard_area(
    canvas: &mut tiny_skia::Pixmap,
    scale: (f32, f32),
    offset: (i32, i32),
    area: tiny_skia::IntRect,
) {
    let full = tiny_skia::IntRect::from_xywh(0, 0, canvas.width(), canvas.height()).unwrap();
    let Some(area) = area.intersect(&full) else {
        return;
    };
    let stride = canvas.width() as usize * 4;
    let colors = [[51, 55, 62, 255], [60, 64, 72, 255]];
    let cell = ((12.0 * scale.0).max(1.0), (12.0 * scale.1).max(1.0));
    let rows = [0_usize, 1].map(|phase| {
        (area.left()..area.right())
            .flat_map(|column| {
                colors[(((column as f32 + offset.0 as f32) / cell.0).floor() as i32 + phase as i32)
                    .rem_euclid(2) as usize]
            })
            .collect::<Vec<_>>()
    });
    let bytes = area.width() as usize * 4;
    for row in area.top()..area.bottom() {
        let first = row as usize * stride + area.left() as usize * 4;
        canvas.data_mut()[first..first + bytes].copy_from_slice(
            &rows
                [(((row as f32 + offset.1 as f32) / cell.1).floor() as i32).rem_euclid(2) as usize],
        );
    }
}
