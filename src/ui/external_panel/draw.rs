//! 【外部动作面板】【列表绘制】原版侧边框、标题、应用图标与滚动列表，正文不绘入导出图像

use super::{ExternalPanel, Kind, layout::ROW};
use crate::{
    features::external_actions::Action,
    renderer::{Painter, text::TextStyle},
    ui::panel::UiPanel,
};
use tiny_skia::Rect;

/// 【外部动作面板】【缓冲绘制】把 panel 绘入 p，复用面板缓存；无返回值
pub fn draw(p: &mut Painter, panel: &mut ExternalPanel) {
    let Some(rect) = panel.rect() else { return };
    let mut cache = panel.pixmap.take();
    p.panel_unframed(
        &mut cache,
        (rect.left(), rect.top()),
        panel.size(),
        1.0,
        panel.dirty,
        |p| content(p, panel),
    );
    panel.pixmap = cache;
    panel.dirty = false;
}

/// 【外部动作面板】【文字省略】按 p、text、width 和 style 返回能放入单行的标题，保留完整 Unicode 字素
fn elide(p: &mut Painter, text: &str, width: f32, style: TextStyle) -> String {
    if p.measure_styled(text, style) <= width {
        return text.to_owned();
    }
    use unicode_segmentation::UnicodeSegmentation;
    let parts: Vec<_> = text.grapheme_indices(true).collect();
    let (mut left, mut right) = (0, parts.len());
    while left < right {
        let middle = (left + right).div_ceil(2);
        let end = parts.get(middle).map_or(text.len(), |part| part.0);
        if p.measure_styled(&format!("{}…", &text[..end]), style) <= width {
            left = middle;
        } else {
            right = middle - 1;
        }
    }
    format!(
        "{}…",
        &text[..parts.get(left).map_or(text.len(), |part| part.0)]
    )
}

/// 【外部动作面板】【提示换行】按 p、message、width 和 style 返回换行文本，长路径也能按 Unicode 字素断行
pub fn wrap(p: &mut Painter, message: &str, width: f32, style: TextStyle) -> Vec<String> {
    use unicode_segmentation::UnicodeSegmentation;
    let mut lines = Vec::new();
    for paragraph in message.split('\n') {
        let mut line = String::new();
        for grapheme in paragraph.graphemes(true) {
            let candidate = format!("{line}{grapheme}");
            if !line.is_empty() && p.measure_styled(&candidate, style) > width {
                lines.push(std::mem::take(&mut line));
            }
            line.push_str(grapheme);
        }
        lines.push(line);
    }
    lines
}

/// 【外部动作面板】【内容绘制】把 panel 的标题、可见行及滚动条绘入 p，缓存仅保存小面板像素，无返回值
fn content(p: &mut Painter, panel: &mut ExternalPanel) {
    let (Some(layout), Some(kind)) = (panel.layout, panel.kind) else {
        return;
    };
    let frame = Rect::from_xywh(0.5, 0.5, layout.size.0 - 1.0, layout.size.1 - 1.0).unwrap();
    p.fill_rounded(
        frame,
        11.0,
        crate::theme::toolbar::Palette::current().popup_fill,
    );
    crate::ui::text_properties::draw::border(
        p,
        frame,
        11.0,
        1.0,
        crate::theme::toolbar::Palette::current().popup_border,
    );
    let language = crate::ui::language::Language::current();
    let label = TextStyle::new(10.0, crate::theme::toolbar::Palette::current().muted_text)
        .weight(cosmic_text::Weight::SEMIBOLD);
    let title = Rect::from_xywh(
        layout.title.left() + 3.0,
        layout.title.top(),
        layout.title.width() - 6.0,
        layout.title.height() - 3.0,
    )
    .unwrap();
    p.text(title, language.text(kind.title()), label);
    // 1. 【外部动作面板】【列表隔离】部分行文字和图标只绘入列表区域，不覆盖标题或面板圆角
    p.clipped_scene(Some(layout.list), |p| {
        if panel.actions.is_empty() {
            for (index, line) in wrap(p, &panel.message, layout.list.width() - 6.0, label)
                .iter()
                .enumerate()
            {
                let area = Rect::from_xywh(
                    layout.list.left() + 3.0,
                    layout.list.top() + index as f32 * 15.0,
                    layout.list.width() - 6.0,
                    15.0,
                )
                .unwrap();
                p.text(area, line, label);
            }
            return;
        }
        let first = (panel.offset / ROW).floor() as usize;
        let last = ((panel.offset + layout.list.height()) / ROW).ceil() as usize;
        let scrollbar = panel.max_offset() > 0.0;
        for index in first..last.min(panel.actions.len()) {
            let top = layout.list.top() + index as f32 * ROW - panel.offset;
            let cell = Rect::from_xywh(
                layout.list.left() + 0.5,
                top + 1.5,
                layout.list.width() - if scrollbar { 13.0 } else { 1.0 },
                ROW - 3.0,
            )
            .unwrap();
            let selected = panel.active == Some(index);
            if selected || panel.hovered == Some(index) {
                p.fill_rounded(
                    cell,
                    7.0,
                    if selected {
                        crate::theme::toolbar::Palette::current().hover_fill
                    } else {
                        crate::theme::toolbar::Palette::current().soft_hover_fill
                    },
                );
                crate::ui::text_properties::draw::border(
                    p,
                    cell,
                    7.0,
                    1.0,
                    if selected {
                        crate::theme::toolbar::Palette::current().hover_border
                    } else {
                        crate::theme::toolbar::Palette::current().soft_hover_border
                    },
                );
            }
            let action = &panel.actions[index];
            let mut left = cell.left() + 9.0;
            if kind == Kind::OpenWith {
                let icon = if let Action::Desktop(app) = action {
                    panel.icons.get(&app.icon)
                } else {
                    None
                };
                if let Some(icon) = icon {
                    let scale = p.scale();
                    let resized = icon.as_ref();
                    let transform = tiny_skia::Transform::from_scale(scale, scale)
                        .post_translate(left * scale, (top + (ROW - 22.0) / 2.0) * scale);
                    let origin = p.origin();
                    p.canvas().draw_pixmap(
                        0,
                        0,
                        resized,
                        &Default::default(),
                        transform.post_translate(-origin.0 * scale, -origin.1 * scale),
                        None,
                    );
                    left += 26.0;
                }
            }
            let text =
                Rect::from_xywh(left, top, (cell.right() - left - 9.0).max(1.0), ROW).unwrap();
            let style = TextStyle::new(
                16.0,
                if selected {
                    crate::theme::toolbar::Palette::current().hover_text
                } else {
                    crate::theme::toolbar::Palette::current().text
                },
            );
            let name = elide(p, action.name(), text.width(), style);
            p.text(text, &name, style);
        }
    });
    if let Some(thumb) = panel.thumb() {
        p.fill_rounded(
            thumb,
            4.0,
            crate::theme::toolbar::Palette::current().grip_dot,
        );
    }
}
