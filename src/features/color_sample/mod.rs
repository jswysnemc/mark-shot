//! 【取色标注】【持久模型】保存取样点、标签位置和颜色，预览与导出共用几何

pub mod draw;
pub(crate) mod edit;
pub mod formats;
pub mod inspector;
pub mod inspector_draw;
pub(crate) mod sampling;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorSample {
    pub source: (f32, f32),
    pub label: (f32, f32),
    pub color: tiny_skia::Color,
    pub size: f32,
}

impl ColorSample {
    /// 【取色标注】【标签范围】按自身位置和字号返回包含色块及十六进制文本的外框，无参数
    pub fn label_rect(self) -> tiny_skia::Rect {
        tiny_skia::Rect::from_xywh(self.label.0, self.label.1, self.size * 9.0, self.size * 2.2)
            .unwrap()
    }

    /// 【取色标注】【完整范围】无参数，返回包含取样点、引线和标签的重绘范围
    pub fn bounds(self) -> tiny_skia::Rect {
        let label = self.label_rect();
        tiny_skia::Rect::from_ltrb(
            label.left().min(self.source.0 - 5.0),
            label.top().min(self.source.1 - 5.0),
            label.right().max(self.source.0 + 5.0),
            label.bottom().max(self.source.1 + 5.0),
        )
        .unwrap()
    }

    /// 【取色标注】【坐标映射】按 map 映射取样点与标签，返回新标注；颜色和文字尺寸保持原值
    pub fn mapped(self, map: impl Fn((f32, f32)) -> (f32, f32)) -> Self {
        Self {
            source: map(self.source),
            label: map(self.label),
            ..self
        }
    }

    /// 【取色标注】【导出缩放】按 scale 映射坐标和字体，返回同一原图颜色的标注
    pub fn scaled(self, scale: f32) -> Self {
        Self {
            size: self.size * scale,
            ..self.mapped(|point| (point.0 * scale, point.1 * scale))
        }
    }
}
