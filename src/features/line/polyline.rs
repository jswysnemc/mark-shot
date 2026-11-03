//! 【线条】【弧长计算】对有限中心线计算路径长度、切线、截取和最近投影

use super::Point;

pub struct Polyline {
    pub points: Vec<Point>,
    distances: Vec<f32>,
    pub length: f32,
}

impl Polyline {
    /// 【线条】【中心线创建】从 points 建立累计弧长；退化或非有限中心线返回空值
    pub fn new(points: Vec<Point>) -> Option<Self> {
        let mut clean = Vec::with_capacity(points.len());
        let mut distances = Vec::with_capacity(points.len());
        let mut length = 0.0;
        for point in points {
            if let Some(previous) = clean.last().copied() {
                let distance = distance(previous, point);
                if !distance.is_finite() {
                    return None;
                }
                if distance <= 0.00001 {
                    continue;
                }
                length += distance;
            }
            clean.push(point);
            distances.push(length);
        }
        (clean.len() >= 2 && length.is_finite() && length > 0.00001).then_some(Self {
            points: clean,
            distances,
            length,
        })
    }

    /// 【线条】【弧长定位】返回 distance 距起点的线段索引与线性比例，输入自动约束到中心线范围
    fn location(&self, distance: f32) -> (usize, f32) {
        let target = distance.clamp(0.0, self.length);
        let index = self
            .distances
            .partition_point(|value| *value < target)
            .clamp(1, self.points.len() - 1);
        let before = self.distances[index - 1];
        (index, (target - before) / (self.distances[index] - before))
    }

    /// 【线条】【弧长取点】返回距离起点 distance 的中心线坐标
    pub fn at(&self, distance: f32) -> Point {
        let (index, ratio) = self.location(distance);
        lerp(self.points[index - 1], self.points[index], ratio)
    }

    /// 【线条】【方向取样】返回距离起点 distance 的单位切线，退化段使用整条中心线方向
    pub fn tangent(&self, distance: f32) -> Point {
        let (index, _) = self.location(distance);
        normalized(sub(self.points[index], self.points[index - 1]))
    }

    /// 【线条】【中心线截取】返回弧长 start 到 end 之间的中心线点，保留精确边界坐标
    pub fn section(&self, start: f32, end: f32) -> Vec<Point> {
        let start = start.clamp(0.0, self.length);
        let end = end.clamp(start, self.length);
        let mut result = vec![self.at(start)];
        for (point, position) in self.points.iter().zip(&self.distances) {
            if *position > start && *position < end {
                result.push(*point);
            }
        }
        result.push(self.at(end));
        result
    }

    /// 【线条】【等弧长取样】返回至多 count 段的均匀中心线，端点保持精确，count 最多五百一十二
    pub fn resampled(&self, count: usize) -> Self {
        let count = count.clamp(2, 512);
        let points = (0..=count)
            .map(|index| self.at(self.length * index as f32 / count as f32))
            .collect();
        Self::new(points).expect("a finite nondegenerate polyline preserves its endpoints")
    }

    /// 【线条】【路径投影】返回 point 的最近投影、距离和弧长比例；整条中心线必须已经有效
    pub fn nearest(&self, point: Point) -> (Point, f32, f32) {
        let mut best = (self.points[0], f32::INFINITY, 0.0);
        for index in 1..self.points.len() {
            let start = self.points[index - 1];
            let end = self.points[index];
            let delta = sub(end, start);
            let squared = delta.0 * delta.0 + delta.1 * delta.1;
            let offset = sub(point, start);
            let ratio = ((offset.0 * delta.0 + offset.1 * delta.1) / squared).clamp(0.0, 1.0);
            let projected = lerp(start, end, ratio);
            let distance = distance(point, projected);
            if distance < best.1 {
                let position = self.distances[index - 1]
                    + ratio * (self.distances[index] - self.distances[index - 1]);
                best = (projected, distance, position / self.length);
            }
        }
        best
    }
}

/// 【线条】【向量差】返回 first 减去 second 的二维向量
pub fn sub(first: Point, second: Point) -> Point {
    (first.0 - second.0, first.1 - second.1)
}

/// 【线条】【线性插值】返回 first 到 second 在 ratio 处的坐标
pub fn lerp(first: Point, second: Point, ratio: f32) -> Point {
    (
        first.0 + (second.0 - first.0) * ratio,
        first.1 + (second.1 - first.1) * ratio,
    )
}

/// 【线条】【两点距离】返回 first 与 second 的欧氏距离，避免平方和溢出
pub fn distance(first: Point, second: Point) -> f32 {
    (first.0 - second.0).hypot(first.1 - second.1)
}

/// 【线条】【单位方向】把 vector 归一化；退化或非法方向返回水平单位向量
pub fn normalized(vector: Point) -> Point {
    let length = vector.0.hypot(vector.1);
    if length.is_finite() && length > 0.00001 {
        (vector.0 / length, vector.1 / length)
    } else {
        (1.0, 0.0)
    }
}

/// 【线条】【法线方向】返回 vector 的左侧法线，不改变其长度
pub fn normal(vector: Point) -> Point {
    (-vector.1, vector.0)
}
