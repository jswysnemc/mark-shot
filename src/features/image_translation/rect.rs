//! 【图片翻译】【图像坐标】使用双精度保留原版矩形计算，绘制时才转换逻辑像素

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BoxRect(pub [f64; 4]);

impl BoxRect {
    /// 【图片翻译】【矩形构造】按 x、y、width、height 返回图像矩形，不隐式修复无效输入
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self([x, y, width, height])
    }
    /// 【图片翻译】【左边坐标】返回当前矩形左边，无参数
    pub fn left(self) -> f64 {
        self.0[0]
    }
    /// 【图片翻译】【上边坐标】返回当前矩形上边，无参数
    pub fn top(self) -> f64 {
        self.0[1]
    }
    /// 【图片翻译】【矩形宽度】返回当前矩形宽度，无参数
    pub fn width(self) -> f64 {
        self.0[2]
    }
    /// 【图片翻译】【矩形高度】返回当前矩形高度，无参数
    pub fn height(self) -> f64 {
        self.0[3]
    }
    /// 【图片翻译】【右边坐标】返回连续坐标中的右边，无参数，不采用整数矩形的包含端点
    pub fn right(self) -> f64 {
        self.left() + self.width()
    }
    /// 【图片翻译】【下边坐标】返回连续坐标中的下边，无参数
    pub fn bottom(self) -> f64 {
        self.top() + self.height()
    }
    /// 【图片翻译】【有效资格】返回矩形是否有限且具有正面积，无参数
    pub fn valid(self) -> bool {
        self.0.iter().all(|v| v.is_finite()) && self.width() > 0.0 && self.height() > 0.0
    }
    /// 【图片翻译】【合并范围】按 other 返回包围两个有效矩形的最小范围，无效矩形不扩大范围
    pub fn union(self, other: Self) -> Self {
        if !self.valid() {
            return other;
        }
        if !other.valid() {
            return self;
        }
        let (x, y) = (self.left().min(other.left()), self.top().min(other.top()));
        Self::new(
            x,
            y,
            self.right().max(other.right()) - x,
            self.bottom().max(other.bottom()) - y,
        )
    }
    /// 【图片翻译】【范围相交】按 other 返回严格正面积相交矩形，仅接触边界或无效输入返回空矩形
    pub fn intersect(self, other: Self) -> Self {
        if !self.valid() || !other.valid() {
            return Self::default();
        }
        let (x, y) = (self.left().max(other.left()), self.top().max(other.top()));
        let result = Self::new(
            x,
            y,
            self.right().min(other.right()) - x,
            self.bottom().min(other.bottom()) - y,
        );
        if result.valid() {
            result
        } else {
            Self::default()
        }
    }
    /// 【图片翻译】【范围平移】按 offset 返回平移后的矩形，保留原始大小
    pub fn shifted(self, offset: (f64, f64)) -> Self {
        Self::new(
            self.left() + offset.0,
            self.top() + offset.1,
            self.width(),
            self.height(),
        )
    }
    /// 【图片翻译】【范围扩展】按 padding 向四边扩展，返回连续坐标矩形
    pub fn expanded(self, padding: f64) -> Self {
        Self::new(
            self.left() - padding,
            self.top() - padding,
            self.width() + padding * 2.0,
            self.height() + padding * 2.0,
        )
    }
    /// 【图片翻译】【绘制范围】返回单精度绘制矩形，无参数，无效范围返回空值
    pub fn draw_rect(self) -> Option<tiny_skia::Rect> {
        self.valid()
            .then(|| {
                tiny_skia::Rect::from_xywh(
                    self.left() as f32,
                    self.top() as f32,
                    self.width() as f32,
                    self.height() as f32,
                )
            })
            .flatten()
    }
}
