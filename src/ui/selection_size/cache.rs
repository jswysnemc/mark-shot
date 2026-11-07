//! 【选区尺寸】【像素缓存】字号按逻辑尺寸布局，缓冲按设备比例分配，静止状态不重建文字或图标

use super::{draw, field_draw, ratio_popup::Popup, state::Panel};
use crate::{renderer::Painter, ui::language::Language};
use tiny_skia::{Pixmap, Rect, Transform};

pub struct Overlay<'a> {
    pub rect: Rect,
    pub pixels: &'a Pixmap,
    pub under_toolbars: bool,
}

/// 【选区尺寸】【尺寸缓存】按 panel、scale、popup_open 和 caret_visible 更新本地像素，返回是否成功分配
pub fn panel(panel: &mut Panel, scale: f32, popup_open: bool, caret_visible: bool) -> bool {
    let (width, height) = (
        (panel.layout.size.0 * scale).ceil() as u32,
        (panel.layout.size.1 * scale).ceil() as u32,
    );
    let fresh = panel
        .cache
        .as_ref()
        .is_none_or(|cache| cache.width() != width || cache.height() != height);
    if !fresh && !panel.dirty {
        return true;
    }
    let mut pixels = if fresh {
        Pixmap::new(width, height)
    } else {
        panel.cache.take()
    };
    let Some(canvas) = pixels.as_mut() else {
        return false;
    };
    canvas.fill(tiny_skia::Color::TRANSPARENT);
    let shapes = std::array::from_fn(|index| {
        panel.fields[index].editing.then(|| {
            field_draw::shape(
                &panel.fields[index],
                panel.layout.fields[index],
                &mut panel.fonts,
                &mut panel.swash,
            )
        })
    });
    // 1. 【选区尺寸】【独立借用】内容与字库分别借用，不复制字段或重新扫描字体
    let content = draw::Content {
        layout: panel.layout,
        fields: &panel.fields,
        ratio: panel.ratio,
        language: panel.language,
        hovered: panel.hovered,
    };
    draw::panel(
        &mut Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut panel.fonts, &mut panel.swash),
        &content,
        &shapes,
        popup_open,
        caret_visible,
    );
    panel.cache = pixels;
    panel.dirty = false;
    true
}

/// 【选区尺寸】【比例缓存】按 popup、panel、scale 和 language 重建列表像素，返回是否成功分配
pub fn popup(popup: &mut Popup, panel: &mut Panel, scale: f32, language: Language) -> bool {
    let (width, height) = (
        (popup.rect.width() * scale).ceil() as u32,
        (popup.rect.height() * scale).ceil() as u32,
    );
    let fresh = popup
        .cache
        .as_ref()
        .is_none_or(|cache| cache.width() != width || cache.height() != height);
    if !fresh && !popup.dirty {
        return true;
    }
    let mut pixels = Pixmap::new(width, height);
    let Some(canvas) = pixels.as_mut() else {
        return false;
    };
    draw::popup(
        &mut Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut panel.fonts, &mut panel.swash),
        popup,
        language,
    );
    popup.cache = pixels;
    popup.dirty = false;
    true
}

/// 【选区尺寸】【缓存合成】按 overlay 在 p 的当前坐标系统合成像素，dirty 限制损坏范围，不重排字段，无返回值
pub fn composite(p: &mut Painter, overlay: &Overlay, dirty: Option<Rect>) {
    p.clipped_scene(dirty, |p| {
        let origin = p.origin();
        let scale = p.scale();
        let transform = Transform::from_scale(
            overlay.rect.width() * scale / overlay.pixels.width() as f32,
            overlay.rect.height() * scale / overlay.pixels.height() as f32,
        )
        .post_translate(
            (overlay.rect.left() - origin.0) * scale,
            (overlay.rect.top() - origin.1) * scale,
        );
        p.canvas().draw_pixmap(
            0,
            0,
            overlay.pixels.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            transform,
            None,
        );
    });
}
