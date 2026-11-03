//! 【贴图】【输出放置】完整逻辑图像与可见表面分离，跨输出时按相交面积选择目标

use super::resize::MIN_EXTENT;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Geometry {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}

impl Geometry {
    /// 【贴图】【几何构造】由 position 和 size 构造全局逻辑矩形，不分配图像
    pub fn new(position: (i32, i32), size: (u32, u32)) -> Self {
        Self {
            left: position.0,
            top: position.1,
            width: size.0,
            height: size.1,
        }
    }
    /// 【贴图】【横向边界】返回排他右边界，使用宽整数避免负坐标和大图溢出
    pub fn right(self) -> i64 {
        i64::from(self.left) + i64::from(self.width)
    }
    /// 【贴图】【纵向边界】返回排他下边界，使用宽整数避免负坐标和大图溢出
    pub fn bottom(self) -> i64 {
        i64::from(self.top) + i64::from(self.height)
    }
    /// 【贴图】【可见交集】返回与 other 相交的全局几何，不相交时宽高为零
    pub fn intersection(self, other: Self) -> Self {
        let left = self.left.max(other.left);
        let top = self.top.max(other.top);
        Self::new(
            (left, top),
            (
                (self.right().min(other.right()) - i64::from(left)).max(0) as u32,
                (self.bottom().min(other.bottom()) - i64::from(top)).max(0) as u32,
            ),
        )
    }
    /// 【贴图】【中心距离】返回与 other 中心距离平方，用于相交面积相等时选择较近输出
    fn distance(self, other: Self) -> f64 {
        let x = f64::from(self.left) + f64::from(self.width) / 2.0
            - f64::from(other.left)
            - f64::from(other.width) / 2.0;
        let y = f64::from(self.top) + f64::from(self.height) / 2.0
            - f64::from(other.top)
            - f64::from(other.height) / 2.0;
        x * x + y * y
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Placement {
    pub logical: Geometry,
    pub visible: Geometry,
    pub offset: (i32, i32),
    pub margin: (i32, i32),
}

/// 【贴图】【目标输出】按 geometry 与 screens 的交集面积、中心距离返回唯一索引；空列表返回空值
pub fn best_output(geometry: Geometry, screens: &[Geometry]) -> Option<usize> {
    let mut best = None;
    let mut area = 0;
    let mut distance = f64::INFINITY;
    for (index, screen) in screens.iter().copied().enumerate() {
        if screen.width == 0 || screen.height == 0 {
            continue;
        }
        let intersection = geometry.intersection(screen);
        let candidate = u64::from(intersection.width) * u64::from(intersection.height);
        let near = geometry.distance(screen);
        if best.is_none() || candidate > area || candidate == area && near < distance {
            best = Some(index);
            area = candidate;
            distance = near;
        }
    }
    best
}

/// 【贴图】【屏幕裁切】将 geometry 保留至少二十四像素可见，返回 screen 内表面、绘制偏移与协议边距
pub fn place(mut geometry: Geometry, screen: Geometry) -> Placement {
    if geometry.width == 0 || geometry.height == 0 || screen.width == 0 || screen.height == 0 {
        return Placement::default();
    }
    let min_width = MIN_EXTENT.min(geometry.width).min(screen.width);
    let min_height = MIN_EXTENT.min(geometry.height).min(screen.height);
    geometry.left = i64::from(geometry.left)
        .clamp(
            i64::from(screen.left) - i64::from(geometry.width) + i64::from(min_width),
            screen.right() - i64::from(min_width),
        )
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    geometry.top = i64::from(geometry.top)
        .clamp(
            i64::from(screen.top) - i64::from(geometry.height) + i64::from(min_height),
            screen.bottom() - i64::from(min_height),
        )
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    let visible = geometry.intersection(screen);
    Placement {
        logical: geometry,
        visible,
        offset: (
            geometry.left.saturating_sub(visible.left),
            geometry.top.saturating_sub(visible.top),
        ),
        margin: (
            visible.left.saturating_sub(screen.left),
            visible.top.saturating_sub(screen.top),
        ),
    }
}
