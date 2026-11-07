//! 【选区尺寸】【内容绘制】按原版固定布局绘制字段和比例弹层，不把界面写入导出图层

use super::{
    field::NumberField,
    field_draw::{self, Shape},
    glyphs,
    layout::{FONT_SIZE, Layout},
    palette::Colors,
    ratio_popup::Popup,
    state::Hit,
};
use crate::{
    features::selection_size::ratio::{self, Field},
    renderer::{Painter, rounded_rect_path, text::TextStyle},
    ui::language::Language,
};
use cosmic_text::Weight;
use tiny_skia::{Rect, Stroke};

pub(crate) struct Content<'a> {
    pub layout: Layout,
    pub fields: &'a [NumberField; 2],
    pub ratio: f64,
    pub language: Language,
    pub hovered: Option<Hit>,
}

/// 【选区尺寸】【面板背景】在 p 的 bounds 绘制 colors 圆角底板与原版一像素边框，无返回值
fn background(p: &mut Painter, bounds: Rect, colors: Colors) {
    if let Some(inset) = Rect::from_xywh(
        bounds.left() + 0.5,
        bounds.top() + 0.5,
        bounds.width() - 1.0,
        bounds.height() - 1.0,
    ) && let Some(path) = rounded_rect_path(&inset, 7.0, true, true, true, true)
    {
        p.fill_path(&path, colors.fill);
        p.stroke_path(
            &path,
            colors.border.color(),
            &Stroke {
                width: 1.0,
                ..Stroke::default()
            },
        );
    }
}

/// 【选区尺寸】【尺寸内容】在 p 绘制 panel 本地布局，shapes 提供活动字段字形，popup_open 和 caret_visible 控制状态，无返回值
pub(crate) fn panel(
    p: &mut Painter,
    panel: &Content,
    shapes: &[Option<Shape>; 2],
    popup_open: bool,
    caret_visible: bool,
) {
    let colors = Colors::current();
    let layout = panel.layout;
    background(
        p,
        Rect::from_xywh(0.0, 0.0, layout.size.0, layout.size.1).unwrap(),
        colors,
    );
    for (index, field) in [Field::Width, Field::Height].into_iter().enumerate() {
        let bounds = layout.field(field);
        if let Some(shape) = &shapes[index] {
            field_draw::draw(p, bounds, shape, colors, caret_visible);
        } else {
            if panel.hovered == Some(Hit::Field(field)) {
                p.fill_rounded(bounds, 5.0, colors.hover);
            }
            p.text(
                bounds,
                &panel.fields[index].value.to_string(),
                TextStyle::new(FONT_SIZE, colors.text)
                    .weight(Weight(600))
                    .center(),
            );
        }
    }
    // 1. 【选区尺寸】【固定标签】乘号、像素单位与细分隔线使用同一字体和次要前景
    for (bounds, label) in [(layout.times, "×"), (layout.unit, "px")] {
        p.text(
            bounds,
            label,
            TextStyle::new(FONT_SIZE, colors.muted)
                .weight(Weight(600))
                .center(),
        );
    }
    p.fill_rounded(layout.separator, 0.0, colors.separator);
    if panel.hovered == Some(Hit::Lock) {
        p.fill_rounded(layout.lock, 5.0, colors.hover);
    }
    if panel.hovered == Some(Hit::Ratio) || popup_open {
        p.fill_rounded(layout.ratio, 5.0, colors.hover);
    }
    // 2. 【选区尺寸】【锁与比例】原版整数 QRect 中心向下取整，再添加半像素偏移
    let locked = panel.ratio > 0.0;
    let lock_box = Rect::from_xywh(
        layout.lock.left() + (layout.lock.width() - 14.0) / 2.0 - 0.5,
        5.5,
        14.0,
        14.0,
    )
    .unwrap();
    glyphs::lock(
        p,
        lock_box,
        locked,
        if locked { colors.accent } else { colors.muted },
    );
    ratio_trigger(p, layout, panel.ratio, panel.language, colors);
}

/// 【选区尺寸】【比例触发区】按 layout、ratio、language 和 colors 在 p 绘制示意框、预设名称与箭头，无返回值
fn ratio_trigger(p: &mut Painter, layout: Layout, ratio: f64, language: Language, colors: Colors) {
    let bounds = layout.ratio;
    let ink = if ratio > 0.0 {
        colors.accent
    } else {
        colors.text
    };
    glyphs::ratio(
        p,
        Rect::from_xywh(bounds.left() + 5.0, 5.5, 14.0, 14.0).unwrap(),
        ratio,
        ink,
    );
    let label = ratio::preset(ratio).map_or("Custom", |index| ratio::PRESETS[index].0);
    p.text(
        Rect::from_xywh(
            bounds.left() + 24.0,
            bounds.top(),
            bounds.width() - 40.0,
            bounds.height(),
        )
        .unwrap(),
        language.text(label),
        TextStyle::new(FONT_SIZE, ink).weight(Weight(600)),
    );
    glyphs::chevron(
        p,
        Rect::from_xywh(bounds.right() - 15.0, 5.5, 10.0, 14.0).unwrap(),
        colors.muted,
    );
}

/// 【选区尺寸】【比例列表内容】在 p 按 popup 与 language 绘制六行固定预设，活动行和当前行分别着色，无返回值
pub(crate) fn popup(p: &mut Painter, popup: &Popup, language: Language) {
    let colors = Colors::current();
    background(
        p,
        Rect::from_xywh(0.0, 0.0, popup.rect.width(), popup.rect.height()).unwrap(),
        colors,
    );
    for (index, (label, ratio)) in ratio::PRESETS.into_iter().enumerate() {
        let row = popup.row(index);
        let current = popup.current == Some(index);
        if index == popup.active {
            p.fill_rounded(row, 5.0, colors.hover);
        }
        let ink = if current { colors.accent } else { colors.text };
        let top = row.top() + 6.0;
        glyphs::ratio(
            p,
            Rect::from_xywh(row.left() + 8.0, top, 14.0, 14.0).unwrap(),
            ratio,
            ink,
        );
        p.text(
            Rect::from_xywh(
                row.left() + 30.0,
                row.top(),
                row.width() - 56.0,
                row.height(),
            )
            .unwrap(),
            language.text(label),
            TextStyle::new(FONT_SIZE, ink).weight(Weight(600)),
        );
        if current {
            glyphs::check(
                p,
                Rect::from_xywh(row.right() - 23.0, top, 14.0, 14.0).unwrap(),
                colors.accent,
            );
        }
    }
}
