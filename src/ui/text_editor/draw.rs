//! 【界面控件】【文字绘制】按分数缩放绘制并裁切文字、选择与光标

use cosmic_text::{Edit, FontSystem, SwashCache};
use tiny_skia::{Pixmap, PremultipliedColorU8, Rect};

use super::TextEditor;
use crate::theme::color;

/// 【界面控件】【像素合成】将 source 合成到 canvas 的 clip 内；x、y、width、height 使用物理像素
fn blend(
    canvas: &mut Pixmap,
    clip: [i32; 4],
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    source: cosmic_text::Color,
) {
    let alpha = u32::from(source.a());
    if alpha == 0 {
        return;
    }
    let left = x.max(clip[0]).max(0);
    let top = y.max(clip[1]).max(0);
    let right = (x.saturating_add(width as i32))
        .min(clip[2])
        .min(canvas.width() as i32);
    let bottom = (y.saturating_add(height as i32))
        .min(clip[3])
        .min(canvas.height() as i32);
    let stride = canvas.width() as usize;
    let inverse = 255 - alpha;
    let pixels = canvas.pixels_mut();
    for py in top..bottom {
        for px in left..right {
            let index = py as usize * stride + px as usize;
            let destination = pixels[index];
            let channel = |value: u8, previous: u8| {
                ((u32::from(value) * alpha + u32::from(previous) * inverse) / 255) as u8
            };
            if let Some(pixel) = PremultipliedColorU8::from_rgba(
                channel(source.r(), destination.red()),
                channel(source.g(), destination.green()),
                channel(source.b(), destination.blue()),
                (alpha + u32::from(destination.alpha()) * inverse / 255) as u8,
            ) {
                pixels[index] = pixel;
            }
        }
    }
}

impl TextEditor {
    /// 【界面控件】【编辑绘制】用 fonts、cache 将编辑器绘制到 canvas 的 rect；scale 是缓冲比例，focused 控制光标
    pub fn draw(
        &mut self,
        fonts: &mut FontSystem,
        cache: &mut SwashCache,
        canvas: &mut Pixmap,
        rect: Rect,
        scale: f32,
        focused: bool,
    ) {
        self.draw_at(fonts, cache, canvas, rect, scale, focused, 0.0);
    }

    /// 【界面控件】【横向字段】在 rect 内绘制不换行字段，offset 保存横向视区；自动保持光标可见
    pub fn draw_single_line(
        &mut self,
        fonts: &mut FontSystem,
        cache: &mut SwashCache,
        canvas: &mut Pixmap,
        rect: Rect,
        scale: f32,
        focused: bool,
        offset: &mut f32,
    ) {
        self.resize(fonts, rect.width(), rect.height());
        if focused && let Some((x, _)) = self.display_editor(fonts).cursor_position() {
            if x as f32 <= *offset {
                *offset = x.max(0) as f32;
            }
            if x as f32 > *offset + rect.width() - 4.0 {
                *offset = (x as f32 - rect.width() + 4.0).max(0.0);
            }
        }
        self.draw_at(fonts, cache, canvas, rect, scale, focused, *offset);
    }

    /// 【界面控件】【裁切绘制】按 offset 移动画面但保持 rect 裁切范围；其他参数与公共绘制接口一致
    fn draw_at(
        &mut self,
        fonts: &mut FontSystem,
        cache: &mut SwashCache,
        canvas: &mut Pixmap,
        rect: Rect,
        scale: f32,
        focused: bool,
        offset: f32,
    ) {
        self.resize(fonts, rect.width(), rect.height());
        let origin = (
            ((rect.left() - offset) * scale).round() as i32,
            (rect.top() * scale).round() as i32,
        );
        let clip = [
            (rect.left() * scale).round() as i32,
            origin.1,
            (rect.right() * scale).ceil() as i32,
            (rect.bottom() * scale).ceil() as i32,
        ];
        let visible = self
            .preedit()
            .is_none_or(|preedit| preedit.cursor.is_some());
        let cursor = if focused && visible {
            color::caret().cosmic()
        } else {
            cosmic_text::Color::rgba(0, 0, 0, 0)
        };
        crate::renderer::editor_text::draw_editor(
            self.display_editor(fonts),
            fonts,
            cache,
            scale,
            [
                color::foreground().cosmic(),
                cursor,
                color::text_selection().cosmic(),
                color::foreground().cosmic(),
            ],
            |x, y, width, height, color| {
                blend(
                    canvas,
                    clip,
                    origin.0 + x,
                    origin.1 + y,
                    width,
                    height,
                    color,
                )
            },
        );
    }
}
