//! 【滚轮提示】【数值语义】原图尺寸、文字点字号与序号直径分别使用对应单位

use tiny_skia::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Stroke,
    Text,
    Number,
    Mosaic,
    Magnification,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub kind: Kind,
    pub value: f32,
    pub image_scale: f32,
    pub color: Color,
}

impl Sample {
    /// 【滚轮提示】【原图尺寸】无额外参数，返回原图尺寸或文字点字号，序号使用原版气泡直径公式
    pub fn extent(self) -> f32 {
        if self.kind == Kind::Number {
            (26.0 + self.value * 2.7).max(26.0)
        } else {
            self.value
        }
    }

    /// 【滚轮提示】【数值文案】无额外参数，返回原版点或像素单位，放大镜倍率保留两位小数
    pub fn text(self) -> String {
        match self.kind {
            Kind::Text => format!("{} pt", self.extent().round() as i32),
            Kind::Magnification => format!("{:.2} x", self.value),
            _ => format!("{} px", self.extent().round() as i32),
        }
    }

    /// 【滚轮提示】【轮廓资格】无额外参数，返回二十八至九十六窗口像素内的实际轮廓直径；文字与倍率不绘制轮廓
    pub fn footprint(self) -> Option<f32> {
        let extent = self.extent() * self.image_scale;
        (!matches!(self.kind, Kind::Text | Kind::Magnification) && (28.0..=96.0).contains(&extent))
            .then_some(extent)
    }
}
