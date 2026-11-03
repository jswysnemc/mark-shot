//! 【标注】【绘图空间】统一源图范围、原始像素比例和绘图手势的坐标约束

use tiny_skia::Rect;

use crate::editor::EditorState;
use crate::utils::get_full_workspace_rect;

#[derive(Clone, Copy, Debug)]
pub(crate) struct DrawingSpace {
    pub bounds: Rect,
    pub pixel_scale: f32,
}

impl DrawingSpace {
    /// 【标注】【空间读取】根据 state 的原图视口或桌面返回绘图范围及原始像素比例；无有效范围时返回空值
    pub fn from_editor(state: &EditorState) -> Option<Self> {
        if let Some(file) = &state.file_canvas {
            return Some(Self {
                bounds: file.viewport,
                pixel_scale: file.original.width() as f32 / file.viewport.width(),
            });
        }
        let bounds = get_full_workspace_rect(&state.placements)?;
        Some(Self {
            bounds,
            pixel_scale: crate::renderer::capture_scale(&state.captures, &state.placements, bounds),
        })
    }

    /// 【标注】【起点命中】将范围内的有限 global 转为逻辑坐标；范围外不创建草稿，返回空值
    pub fn start_point(self, global: (f64, f64)) -> Option<(f32, f32)> {
        if !global.0.is_finite()
            || !global.1.is_finite()
            || global.0 < f64::from(self.bounds.left())
            || global.0 > f64::from(self.bounds.right())
            || global.1 < f64::from(self.bounds.top())
            || global.1 > f64::from(self.bounds.bottom())
        {
            return None;
        }
        Some((global.0 as f32, global.1 as f32))
    }

    /// 【标注】【终点约束】将有限 global 限定在源图范围并返回逻辑坐标；非有限坐标返回空值
    pub fn clamp_point(self, global: (f64, f64)) -> Option<(f32, f32)> {
        if !global.0.is_finite() || !global.1.is_finite() {
            return None;
        }
        Some((
            (global.0 as f32).clamp(self.bounds.left(), self.bounds.right()),
            (global.1 as f32).clamp(self.bounds.top(), self.bounds.bottom()),
        ))
    }

    /// 【标注】【等比例约束】根据 start、end 和 keep_aspect 返回矩形终点；等比例时缩短到源图可用范围
    pub fn rectangle_end(
        self,
        start: (f32, f32),
        end: (f32, f32),
        keep_aspect: bool,
    ) -> (f32, f32) {
        if !keep_aspect {
            return end;
        }
        let dx = end.0 - start.0;
        let dy = end.1 - start.1;
        let sx = if dx < 0.0 { -1.0 } else { 1.0 };
        let sy = if dy < 0.0 { -1.0 } else { 1.0 };
        let available_x = if sx < 0.0 {
            start.0 - self.bounds.left()
        } else {
            self.bounds.right() - start.0
        };
        let available_y = if sy < 0.0 {
            start.1 - self.bounds.top()
        } else {
            self.bounds.bottom() - start.1
        };
        let side = dx
            .abs()
            .max(dy.abs())
            .min(available_x)
            .min(available_y)
            .max(0.0);
        (start.0 + sx * side, start.1 + sy * side)
    }

    /// 【标注】【最小尺寸】判断 start、end 构成的矩形两边是否均达到 minimum 原始像素；返回布尔值
    pub fn rectangle_measures(self, start: (f32, f32), end: (f32, f32), minimum: f32) -> bool {
        (end.0 - start.0).abs() * self.pixel_scale >= minimum
            && (end.1 - start.1).abs() * self.pixel_scale >= minimum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【空间测试】【空间构造】返回带负坐标偏移和分数像素比例的绘图范围
    fn space() -> DrawingSpace {
        DrawingSpace {
            bounds: Rect::from_xywh(-80.0, 20.0, 100.0, 100.0).unwrap(),
            pixel_scale: 1.5,
        }
    }

    /// 【空间测试】【起点限制】源图外和非有限坐标不创建草稿，拖动终点仍可约束到边界
    #[test]
    fn starts_require_containment_but_endpoints_can_be_clamped() {
        let space = space();
        assert_eq!(space.start_point((-81.0, 40.0)), None);
        assert_eq!(space.start_point((0.0, f64::NAN)), None);
        assert_eq!(space.start_point((-40.0, 60.0)), Some((-40.0, 60.0)));
        assert_eq!(space.clamp_point((100.0, -10.0)), Some((20.0, 20.0)));
        assert_eq!(space.clamp_point((f64::INFINITY, 30.0)), None);
    }

    /// 【空间测试】【反向等比】反向拖动保持正方形，并限制在源图范围内
    #[test]
    fn square_constraints_preserve_direction_and_fit_the_source() {
        let space = space();
        assert_eq!(
            space.rectangle_end((-40.0, 60.0), (-75.0, 45.0), true),
            (-75.0, 25.0)
        );
        assert_eq!(
            space.rectangle_end((10.0, 100.0), (20.0, 120.0), true),
            (20.0, 110.0)
        );
        assert_eq!(
            space.rectangle_end((-40.0, 60.0), (-75.0, 45.0), false),
            (-75.0, 45.0)
        );
    }

    /// 【空间测试】【原始像素】最小尺寸按源图像素计算，不按显示器逻辑像素判断
    #[test]
    fn minimum_dimensions_use_original_image_pixels() {
        let space = space();
        assert!(!space.rectangle_measures((0.0, 30.0), (1.0, 60.0), 2.0));
        assert!(space.rectangle_measures((0.0, 30.0), (2.0, 32.0), 2.0));
        assert!(space.rectangle_measures((2.0, 32.0), (0.0, 30.0), 2.0));
    }
}
