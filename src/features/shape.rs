//! 【框形标注】【几何约束】共用圆角路径与等比例端点，编辑预览和导出使用相同参数

use tiny_skia::{Path, Rect, Transform};

/// 【框形标注】【半径校验】将 value 限制到零至二百五十六像素，非有限值返回零
pub fn radius(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 256.0)
    } else {
        0.0
    }
}

/// 【框形标注】【等比端点】按 start、end 的拖动方向返回正方形终点；保留四个象限并使用较长边
pub fn square_end(start: (f32, f32), end: (f32, f32)) -> (f32, f32) {
    let delta = (end.0 - start.0, end.1 - start.1);
    let side = delta.0.abs().max(delta.1.abs());
    (
        start.0 + side.copysign(delta.0),
        start.1 + side.copysign(delta.1),
    )
}

/// 【框形标注】【圆角路径】从 rect、radius、弧度 rotation 和 pivot 返回闭合路径，半径不超过短边一半
pub fn path(rect: Rect, radius: f32, rotation: f32, pivot: (f32, f32)) -> Option<Path> {
    let radius = radius.max(0.0).min(rect.width().min(rect.height()) / 2.0);
    crate::renderer::rounded_rect_path(&rect, radius, true, true, true, true)?.transform(
        Transform::from_rotate_at(rotation.to_degrees(), pivot.0, pivot.1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_drag_preserves_all_quadrants() {
        for end in [(40.0, 20.0), (-40.0, 20.0), (40.0, -20.0), (-40.0, -20.0)] {
            let constrained = square_end((0.0, 0.0), end);
            assert_eq!(constrained.0.abs(), constrained.1.abs());
            assert_eq!(constrained.0.signum(), end.0.signum());
            assert_eq!(constrained.1.signum(), end.1.signum());
        }
    }

    #[test]
    fn radius_clamps_and_leaves_square_bounds_unchanged() {
        let rect = Rect::from_xywh(10.0, 20.0, 100.0, 40.0).unwrap();
        let rounded = path(rect, 256.0, 0.0, (0.0, 0.0)).unwrap();
        assert_eq!(rounded.bounds(), rect);
        assert_eq!(radius(f32::NAN), 0.0);
        assert_eq!(radius(-5.0), 0.0);
    }
}
