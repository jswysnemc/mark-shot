//! 【选区尺寸】【固定布局】以原版十三像素半粗体计算推荐尺寸，数值变化不改变面板宽度

use crate::{features::selection_size::ratio, ui::language::Language};
use cosmic_text::{FontSystem, Style, Weight};
use tiny_skia::Rect;

pub const FONT_SIZE: f32 = 13.0;
pub const HEIGHT: f32 = 26.0;
pub const POPUP_SIZE: (f32, f32) = (128.0, 164.0);

#[derive(Clone, Copy)]
pub struct Layout {
    pub size: (f32, f32),
    pub fields: [Rect; 2],
    pub times: Rect,
    pub unit: Rect,
    pub separator: Rect,
    pub lock: Rect,
    pub ratio: Rect,
}

/// 【选区尺寸】【整数度量】按 fonts 和 text 返回原版整数文字宽度，使用半粗体和六十四分之一像素量化
pub fn width(fonts: &mut FontSystem, text: &str) -> f32 {
    crate::fonts::selection_ui::prepare(fonts, text);
    let (_, width, _) = crate::renderer::text::shape_single_line(
        fonts,
        text,
        FONT_SIZE,
        Weight(600),
        Style::Normal,
    );
    ((width * 64.0).round() / 64.0).round()
}

impl Layout {
    /// 【选区尺寸】【面板布局】按 fonts 和 language 返回固定字段与比例触发区，不按当前数值缩放宽度
    pub fn new(fonts: &mut FontSystem, language: Language) -> Self {
        let field_width = width(fonts, "00000") + 10.0;
        let mut x = 6.0;
        let first = Rect::from_xywh(x, 3.0, field_width, HEIGHT - 6.0).unwrap();
        x += field_width;
        let times = Rect::from_xywh(x, 0.0, width(fonts, "×") + 6.0, HEIGHT).unwrap();
        x += times.width();
        let second = Rect::from_xywh(x, 3.0, field_width, HEIGHT - 6.0).unwrap();
        x += field_width;
        let unit = Rect::from_xywh(x, 0.0, width(fonts, "px") + 6.0, HEIGHT).unwrap();
        x += unit.width();
        let separator = Rect::from_xywh(x + 3.0, 7.0, 1.0, HEIGHT - 14.0).unwrap();
        x += 7.0;
        let lock = Rect::from_xywh(x, 3.0, 22.0, HEIGHT - 6.0).unwrap();
        x += 24.0;
        let custom_width = width(fonts, language.text("Custom"));
        let label_width = ratio::PRESETS
            .iter()
            .map(|(label, _)| width(fonts, language.text(label)))
            .fold(custom_width, f32::max);
        let ratio = Rect::from_xywh(x, 3.0, 44.0 + label_width, HEIGHT - 6.0).unwrap();
        x += ratio.width() + 4.0;
        Self {
            size: (x, HEIGHT),
            fields: [first, second],
            times,
            unit,
            separator,
            lock,
            ratio,
        }
    }

    /// 【选区尺寸】【字段范围】按 field 返回宽度或高度字段的面板本地矩形
    pub fn field(self, field: ratio::Field) -> Rect {
        self.fields[usize::from(field == ratio::Field::Height)]
    }
}
