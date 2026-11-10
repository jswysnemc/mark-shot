//! 【标注多选】【属性布局】标题、宽度、透明度与颜色使用原版紧凑横排，不显示成员专用属性

use crate::ui::text_properties::layout::{Control, TextLayout};
use tiny_skia::Rect;

/// 【标注多选】【标题文字】按count返回当前语言的原版数量标题，参数为有效成员数量
pub(crate) fn title(count: usize) -> String {
    crate::ui::language::Language::current().format("Group %1", &[&count.to_string()])
}

/// 【标注多选】【标题尺寸】按width返回标题框，参数为已测量标签宽度，最小宽度为五十八像素
pub(crate) fn title_rect(width: f32) -> Rect {
    let scale = (crate::ui::toolbar::metrics::font() / 11.0).max(1.0);
    Rect::from_xywh(
        8.0,
        6.0 * scale,
        width.max(58.0),
        crate::ui::toolbar::metrics::button(false) + 2.0,
    )
    .unwrap()
}

/// 【标注多选】【控件组合】按title_width返回宽度、透明度和颜色布局，参数为已测量标题宽度，其他按钮为空值
pub(crate) fn new(title_width: f32) -> TextLayout {
    let mut layout = TextLayout::new(
        crate::ui::toolbar::metrics::button(false),
        crate::ui::toolbar::metrics::font(),
        crate::ui::toolbar::metrics::icon(),
        false,
    );
    let shift = title_rect(title_width).width() + 6.0;
    let moved = |rect: Rect| {
        Rect::from_xywh(rect.x() + shift, rect.y(), rect.width(), rect.height()).unwrap()
    };
    layout.glyphs = layout.glyphs.map(moved);
    layout.values = layout.values.map(moved);
    for rect in &mut layout.controls {
        *rect = rect.map(moved);
    }
    for rect in layout.controls.iter_mut().take(2).flatten() {
        *rect = Rect::from_xywh(rect.x(), 13.0, rect.width(), 18.0).unwrap();
    }
    let right = layout.controls[Control::Color.index()].unwrap().right();
    for control in [Control::Background, Control::Font, Control::Edit] {
        layout.controls[control.index()] = None;
    }
    layout.size.0 = right + 8.0;
    layout
}
