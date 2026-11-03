//! 【贴图】【边缘缩放】复现八方向命中、最小尺寸和对边固定，不依赖窗口协议

use super::placement::Geometry;

pub const MIN_EXTENT: u32 = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Edge {
    /// 【贴图】【方向分解】返回当前方向的横纵符号，负值代表左或上，零代表该轴不参与
    pub fn axes(self) -> (i32, i32) {
        match self {
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
            Self::Top => (0, -1),
            Self::Bottom => (0, 1),
            Self::TopLeft => (-1, -1),
            Self::TopRight => (1, -1),
            Self::BottomLeft => (-1, 1),
            Self::BottomRight => (1, 1),
        }
    }

    /// 【贴图】【边缘命中】按 size、point 和 margin 返回缩放方向；角落优先，远离窗口返回空值
    pub fn at(size: (u32, u32), point: (f64, f64), margin: f64) -> Option<Self> {
        if size.0 == 0
            || size.1 == 0
            || !margin.is_finite()
            || margin <= 0.0
            || !point.0.is_finite()
            || !point.1.is_finite()
            || point.0 < -margin
            || point.1 < -margin
            || point.0 > f64::from(size.0) + margin
            || point.1 > f64::from(size.1) + margin
        {
            return None;
        }
        let left = point.0.abs() <= margin;
        let right = (point.0 - f64::from(size.0)).abs() <= margin;
        let top = point.1.abs() <= margin;
        let bottom = (point.1 - f64::from(size.1)).abs() <= margin;
        match (left, right, top, bottom) {
            (true, _, true, _) => Some(Self::TopLeft),
            (_, true, true, _) => Some(Self::TopRight),
            (true, _, _, true) => Some(Self::BottomLeft),
            (_, true, _, true) => Some(Self::BottomRight),
            (true, _, _, _) => Some(Self::Left),
            (_, true, _, _) => Some(Self::Right),
            (_, _, true, _) => Some(Self::Top),
            (_, _, _, true) => Some(Self::Bottom),
            _ => None,
        }
    }

    /// 【贴图】【嵌边分流】判断 geometry 的当前边是否贴住 screen；贴住时左键改为移动，便于移回屏幕
    pub fn embedded(self, geometry: Geometry, screen: Geometry) -> bool {
        let (x, y) = self.axes();
        x < 0 && geometry.left <= screen.left
            || x > 0 && geometry.right() >= screen.right()
            || y < 0 && geometry.top <= screen.top
            || y > 0 && geometry.bottom() >= screen.bottom()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Resize {
    pub edge: Edge,
    pub start: Geometry,
    pub pointer: (f64, f64),
}

impl Resize {
    /// 【贴图】【等比几何】由 pointer 的累计位移返回新几何；保持起始比例，对边固定，单边另一轴居中
    pub fn geometry(self, pointer: (f64, f64)) -> Geometry {
        let (x, y) = self.edge.axes();
        let width = f64::from(self.start.width.max(1));
        let height = f64::from(self.start.height.max(1));
        let sx = 1.0 + f64::from(x) * (pointer.0 - self.pointer.0) / width;
        let sy = 1.0 + f64::from(y) * (pointer.1 - self.pointer.1) / height;
        let scale = if y == 0 {
            sx
        } else if x == 0 {
            sy
        } else if (sx - 1.0).abs() >= (sy - 1.0).abs() {
            sx
        } else {
            sy
        };
        let scale = if scale.is_finite() { scale } else { 1.0 };
        // 1. 【贴图】【容量约束】限制完整逻辑边长，实际绘制只分配输出内可见缓冲
        let scale = scale
            .max((f64::from(MIN_EXTENT) / width).max(f64::from(MIN_EXTENT) / height))
            .min(1_048_576.0 / width.max(height));
        let target_height = (height * scale).round().max(1.0) as u32;
        let target_width = (f64::from(target_height) * width / height).round().max(1.0) as u32;
        self.anchored((target_width, target_height))
    }

    /// 【贴图】【固定边计算】将 size 置于起始窗口的固定边或中心；返回完整逻辑几何
    pub fn anchored(self, size: (u32, u32)) -> Geometry {
        let (x, y) = self.edge.axes();
        let left = if x < 0 {
            self.start.right() - i64::from(size.0)
        } else if x > 0 {
            i64::from(self.start.left)
        } else {
            (f64::from(self.start.left) + (f64::from(self.start.width) - f64::from(size.0)) / 2.0)
                .round() as i64
        };
        let top = if y < 0 {
            self.start.bottom() - i64::from(size.1)
        } else if y > 0 {
            i64::from(self.start.top)
        } else {
            (f64::from(self.start.top) + (f64::from(self.start.height) - f64::from(size.1)) / 2.0)
                .round() as i64
        };
        Geometry::new(
            (
                left.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
                top.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            ),
            size,
        )
    }
}
