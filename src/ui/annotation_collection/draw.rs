//! 【标注多选】【数量标题】原版只为多选显示数量胶囊，其他通用属性由统一组件绘制

use crate::{
    renderer::Painter,
    ui::settings_panel::{SettingsPanel, SettingsSource},
};

/// 【标注多选】【标题绘制】按panel当前群组来源绘入p，无群组来源不绘制，无返回值
pub(crate) fn title(p: &mut Painter<'_>, panel: &SettingsPanel) {
    let Some(SettingsSource::Collection { .. }) = panel.active_source else {
        return;
    };
    let Some(title) = panel.collection_title.as_ref() else {
        return;
    };
    let rect = super::layout::title_rect(title.cell_width);
    let palette = crate::theme::toolbar::Palette::current();
    let inset = tiny_skia::Rect::from_xywh(
        rect.x() + 0.5,
        rect.y() + 0.5,
        rect.width() - 1.0,
        rect.height() - 1.0,
    )
    .unwrap();
    p.fill_rounded(inset, 7.0, palette.control_fill);
    crate::ui::text_properties::draw::border(p, inset, 7.0, 1.0, palette.control_border);
    let position = (
        rect.x() + (rect.width() - title.width) / 2.0,
        rect.y() + (rect.height() - title.height) / 2.0,
    );
    for (run, line) in title.buffer.layout_runs().zip(&title.lines) {
        let run = cosmic_text::LayoutRun {
            glyphs: &line.glyphs,
            line_w: line.w,
            ..run
        };
        p.text_layout_run(&run, position, palette.strong_text.color());
    }
}
