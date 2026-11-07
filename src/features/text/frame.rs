//! 【文字标注】【矩形数据】存储矩形独立于字形尺寸，缓存只用于当前比例下的绘制损坏范围

use tiny_skia::Rect;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextFrame {
    /// 【文字标注】【存储宽度】为空时使用当前原图度量的自然选框宽度
    pub width: Option<f32>,
    /// 【文字标注】【存储高度】为空时使用自然选框高度，不直接控制字形排版高度
    pub height: Option<f32>,
    /// 【文字标注】【原始定点】保留生产标注的首个定点，矩形为空时用于默认框回退
    pub anchor: Option<(f32, f32)>,
    /// 【文字标注】【绘制范围】当前比例的实际字形范围，不作为旋转中心或存储矩形
    pub painted: Option<Rect>,
}

/// 【文字标注】【矩形映射】按bounds及map返回映射后的轴对齐矩形，无效几何返回空值
fn mapped(bounds: Rect, map: impl Fn((f32, f32)) -> (f32, f32)) -> Option<Rect> {
    let start = map((bounds.left(), bounds.top()));
    let end = map((bounds.right(), bounds.bottom()));
    Rect::from_ltrb(
        start.0.min(end.0),
        start.1.min(end.1),
        start.0.max(end.0),
        start.1.max(end.1),
    )
}

impl TextFrame {
    /// 【文字标注】【比例映射】按scale返回缩放后的宽高、定点及绘制缓存，不改变原始数据
    pub fn scaled(&self, scale: f32) -> Self {
        Self {
            width: self.width.map(|value| value * scale),
            height: self.height.map(|value| value * scale),
            anchor: self.anchor.map(|point| (point.0 * scale, point.1 * scale)),
            painted: self
                .painted
                .and_then(|bounds| mapped(bounds, |point| (point.0 * scale, point.1 * scale))),
        }
    }

    /// 【文字标注】【位置移动】按delta返回移动后的定点及缓存，存储宽高保持不变
    pub fn translated(&self, delta: (f32, f32)) -> Self {
        Self {
            anchor: self
                .anchor
                .map(|point| (point.0 + delta.0, point.1 + delta.1)),
            painted: self
                .painted
                .and_then(|bounds| mapped(bounds, |point| (point.0 + delta.0, point.1 + delta.1))),
            ..self.clone()
        }
    }
}
