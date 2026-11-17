//! 【副工具栏】【面板绘制】使用与主工具栏一致的圆角毛玻璃卡片外观分派绘制全部工具属性控件

use super::{
    SettingsAction, SettingsPanel, SettingsWidget,
    draw_controls::{draw_color_swatch, draw_download, draw_toggle, draw_value_field},
    draw_stepper::{draw_stepper, stepper_display_text},
};
use crate::{
    renderer::{Painter, paths::rounded_rect_path, text::TextStyle},
    theme::toolbar::Palette,
    ui::{color_popover::ColorTarget, language::Language, panel::UiPanel},
};
use tiny_skia::{Color, Rect, Stroke};

const PANEL_RADIUS: f32 = 11.0;

/// 【副工具栏】【缓存入口】按当前面板可见状态与几何合成副工具栏像素缓存，无返回值
pub fn draw_settings_panel(
    p: &mut Painter,
    panel: &mut SettingsPanel,
    current_color: Color,
    outline_color: Color,
) {
    if !panel.visible {
        return;
    }
    let Some(panel_rect) = panel.rect() else {
        return;
    };
    // 1. 预先补充中文界面下可能出现的开关、提示与录屏参数字形，防止冷启动缺字
    if Language::current() == Language::Chinese {
        p.ensure_ui_text(
            "填充长度角度开始录制关闭正在下载正在识别文字拖拽框选其他区域继续识别选择识别引擎与语言资源格式帧率倒计时音频画质编码器光标",
        );
    }

    let mut cache = panel.panel_pixmap.take();
    p.panel_unframed(
        &mut cache,
        (panel_rect.left(), panel_rect.top()),
        panel.size,
        panel.opacity,
        true,
        |p| draw_settings_content(p, panel, current_color, outline_color),
    );
    panel.panel_pixmap = cache;
}

/// 【副工具栏】【本地化文案】将内置属性栏提示文案转换为当前界面语言，返回显示文本
fn localized_label(text: &'static str) -> &'static str {
    if Language::current() == Language::Chinese {
        match text {
            "Drag a new box to read another area" => "拖拽框选其他区域继续识别",
            "Reading text..." => "正在识别文字...",
            "Choose an OCR engine and resources" => "选择识别引擎与语言资源",
            _ => text,
        }
    } else {
        text
    }
}

/// 【副工具栏】【容器与分派】在本地缓存内绘制统一 11px 圆角卡片并逐项绘制工具控件，无返回值
fn draw_settings_content(
    p: &mut Painter,
    panel: &SettingsPanel,
    current_color: Color,
    outline_color: Color,
) {
    if panel.text_layout.is_some() {
        crate::ui::text_properties::draw::draw(p, panel, current_color);
        return;
    }

    let palette = Palette::current();
    let (w, h) = panel.size;
    let Some(frame) = Rect::from_xywh(0.5, 0.5, (w - 1.0).max(1.0), (h - 1.0).max(1.0)) else {
        return;
    };

    // 1. 绘制与主工具栏及文字属性栏完全统一的圆角底板与一像素外框
    p.fill_rounded(frame, PANEL_RADIUS, palette.panel_fill);
    if let Some(path) = rounded_rect_path(&frame, PANEL_RADIUS, true, true, true, true) {
        p.stroke_path(
            &path,
            palette.panel_border,
            &Stroke {
                width: 1.0,
                ..Default::default()
            },
        );
    }

    // 2. 按网格单元逐项分派各工具属性控件绘制
    for (index, item) in panel.widgets.iter().enumerate() {
        let Some(cell) = panel.cell(index) else {
            continue;
        };
        let is_hovered = panel.hovered == Some(index);
        let is_selected = panel.selected == Some(index);

        match item {
            SettingsWidget::TextControl(_) => {}
            SettingsWidget::ColorSwatch(target) => {
                let color = match target {
                    ColorTarget::Main => current_color,
                    ColorTarget::Outline => outline_color,
                    ColorTarget::TextBackground => panel.text_background,
                };
                draw_color_swatch(p, cell, is_hovered, is_selected, color);
            }
            SettingsWidget::Action { action, svg } => {
                draw_action_widget(p, panel, *action, svg, cell, is_hovered, is_selected);
            }
            SettingsWidget::Stepper { label, unit, .. } => {
                let display_text = stepper_display_text(panel, index, label, unit);
                draw_stepper(
                    p,
                    cell,
                    panel,
                    index,
                    &display_text,
                    is_hovered,
                    is_selected,
                );
            }
            SettingsWidget::Toggle { visual, field } => {
                let is_on = panel.is_toggled(index);
                draw_toggle(
                    p,
                    cell,
                    visual,
                    field.icon_size() * crate::ui::toolbar::metrics::properties_scale(),
                    is_on,
                    is_hovered,
                );
            }
            SettingsWidget::Label(text) => {
                p.text(
                    cell,
                    localized_label(text),
                    TextStyle::new(
                        crate::ui::toolbar::metrics::font(),
                        palette.muted_text.color(),
                    )
                    .center(),
                );
            }
            SettingsWidget::Value { field } => {
                draw_value_field(p, cell, &field.text(current_color), is_hovered);
            }
            SettingsWidget::Download => {
                draw_download(p, cell, panel.download.unwrap_or(0));
            }
            SettingsWidget::Separator => {
                let sep_h = (cell.height() * 0.56).clamp(14.0, 20.0);
                let sep_x = (cell.left() + cell.width() * 0.5).floor();
                let sep_y = cell.top() + (cell.height() - sep_h) * 0.5;
                if let Some(sep_rect) = Rect::from_xywh(sep_x, sep_y, 1.0, sep_h) {
                    p.fill_rounded(sep_rect, 0.5, palette.separator);
                }
            }
        }
    }
}

/// 【副工具栏】【动作控件】区分录屏控件、下拉选择芯片与纯图标幽灵按钮进行绘制，无返回值
fn draw_action_widget(
    p: &mut Painter,
    panel: &SettingsPanel,
    action: SettingsAction,
    svg: &'static str,
    cell: Rect,
    is_hovered: bool,
    is_selected: bool,
) {
    if super::recording::draw(p, panel, action, cell, is_hovered, is_selected) {
        return;
    }
    super::action_icon::draw(p, panel, action, svg, cell, is_hovered, is_selected);
}
