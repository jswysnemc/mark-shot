//! 【属性取色】【面板绘制】原版取色器与透明环形色板共用缓存交接，不共用布局

use super::{ColorPickerPopover, hex_field, layout, swatches, tracks};
use crate::{renderer::Painter, theme::Rgba, ui::panel::UiPanel};
use tiny_skia::{Pixmap, PixmapPaint, Rect, Transform};

/// 【属性取色】【缓存交接】把 picker 当前模式绘入 p，内容变化时才重建缓存，不创建刷新计时器
pub fn draw_color_popover(p: &mut Painter, picker: &mut ColorPickerPopover) {
    let Some(rect) = picker.rect() else {
        return;
    };
    let mut cache = picker.colorpicker_pixmap.take();
    p.panel_unframed(
        &mut cache,
        (rect.left(), rect.top()),
        picker.size,
        picker.opacity,
        picker.dirty,
        |p| {
            if let Some(radial) = &picker.radial {
                let color = picker.sv_square.color().to_color_u8();
                crate::ui::radial_palette::draw(
                    p,
                    radial,
                    Rgba(color.red(), color.green(), color.blue(), color.alpha()),
                );
            } else {
                content(p, picker);
            }
        },
    );
    picker.colorpicker_pixmap = cache;
}

/// 【属性取色】【子控件合成】将 picker 绘入 p；历史色块按实际 Qt 子控件边界裁切，不扩张面板
fn content(p: &mut Painter, picker: &mut ColorPickerPopover) {
    let rect = Rect::from_xywh(0.5, 0.5, layout::WIDTH - 1.0, layout::HEIGHT - 1.0).unwrap();
    p.fill_rounded(
        rect,
        11.0,
        crate::theme::toolbar::Palette::current().popup_fill,
    );
    tracks::border(
        p,
        rect,
        11.0,
        crate::theme::toolbar::Palette::current().popup_border,
    );
    // 1. 【属性取色】【子控件边界】在独立取色器缓冲绘制，保留 Qt 历史色块的重叠与末项裁切
    let scale = p.scale();
    let content = layout::content((0.0, 0.0));
    let Some(mut canvas) = Pixmap::new(
        (content.width() * scale).ceil() as u32,
        (content.height() * scale).ceil() as u32,
    ) else {
        return;
    };
    let mut fonts = picker.picker_fonts.take().unwrap_or_default();
    let text = hex_field::shape(picker, &mut fonts.system);
    {
        let mut painter = p
            .sub(&mut canvas)
            .with_origin((8.0, 8.0))
            .with_text(&mut fonts.system, &mut fonts.swash);
        tracks::draw(&mut painter, picker);
        swatches::draw(&mut painter, picker);
        hex_field::draw(&mut painter, picker, &text);
    }
    picker.picker_fonts = Some(fonts);
    // 2. 【属性取色】【图像合成】显示缓冲只用于控件，不参与截图原图或标注导出
    let (x, y) = p.to_px(8.0, 8.0);
    p.canvas().draw_pixmap(
        0,
        0,
        canvas.as_ref(),
        &PixmapPaint::default(),
        Transform::from_translate(x, y),
        None,
    );
}
