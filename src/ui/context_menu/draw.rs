//! 【上下文菜单】【缓存绘制】沿用原版深色玻璃、细边和圆角行，局部重绘不重复叠加透明背景

use super::ContextMenu;
use crate::{renderer::Painter, theme::Rgba, ui::text_properties::draw::border};
use tiny_skia::{Color, IntRect, Pixmap, Rect};

/// 【上下文菜单】【菜单绘制】将 menu 按 p 比例绘入 dirty 指定区域；缓存只在布局、比例或高亮变化时重建，无返回值
pub fn draw(p: &mut Painter, menu: &mut ContextMenu, dirty: Option<&Rect>) {
    prepare(p, menu);
    let Some(pixmap) = menu.pixmap.as_ref() else {
        return;
    };
    let scale = p.scale();
    let size = (pixmap.width(), pixmap.height());
    let (x, y) = (
        (menu.position.0 * scale).round() as i32,
        (menu.position.1 * scale).round() as i32,
    );
    if let Some(dirty) = dirty {
        let left = ((dirty.left() * scale).floor() as i32 - x).clamp(0, size.0 as i32);
        let top = ((dirty.top() * scale).floor() as i32 - y).clamp(0, size.1 as i32);
        let right = ((dirty.right() * scale).ceil() as i32 - x).clamp(0, size.0 as i32);
        let bottom = ((dirty.bottom() * scale).ceil() as i32 - y).clamp(0, size.1 as i32);
        if let Some(rect) = IntRect::from_ltrb(left, top, right, bottom)
            && let Some(cropped) = pixmap.clone_rect(rect)
        {
            p.draw_pixmap(x + left, y + top, &cropped);
        }
    } else {
        p.draw_pixmap(x, y, pixmap);
    }
}

/// 【上下文菜单】【独立缓冲】用 p 的字体和比例更新 menu 自有缓冲，不绘入父画布；原生弹层和画布测试共用排版，无返回值
pub fn prepare(p: &mut Painter, menu: &mut ContextMenu) {
    let scale = p.scale();
    let size = (
        (menu.layout.size.0 * scale).ceil() as u32,
        (menu.layout.size.1 * scale).ceil() as u32,
    );
    let fresh = menu
        .pixmap
        .as_ref()
        .is_none_or(|pixmap| (pixmap.width(), pixmap.height()) != size);
    if fresh {
        menu.pixmap = Pixmap::new(size.0, size.1);
    }
    let Some(mut pixmap) = menu.pixmap.take() else {
        return;
    };
    if fresh || menu.dirty {
        pixmap.fill(Color::TRANSPARENT);
        content(&mut p.sub(&mut pixmap), menu);
        menu.dirty = false;
    }
    menu.pixmap = Some(pixmap);
}

/// 【上下文菜单】【本地内容】按 menu 布局绘入 p，文案和快捷键使用相同字体，分隔线无交互背景，无返回值
fn content(p: &mut Painter, menu: &ContextMenu) {
    let frame =
        Rect::from_xywh(0.5, 0.5, menu.layout.size.0 - 1.0, menu.layout.size.1 - 1.0).unwrap();
    p.fill_rounded(frame, 8.0, Rgba(24, 27, 35, 246));
    border(p, frame, 8.0, 1.0, Rgba(255, 255, 255, 30));
    for (index, ((row, rect), text)) in menu
        .rows
        .iter()
        .zip(&menu.layout.rows)
        .zip(&menu.layout.text)
        .enumerate()
    {
        if row.separator() {
            p.fill_rounded(
                Rect::from_xywh(
                    rect.left() + 8.0,
                    rect.top() + 4.0,
                    rect.width() - 16.0,
                    1.0,
                )
                .unwrap(),
                0.0,
                Rgba(255, 255, 255, 22),
            );
            continue;
        }
        let active = row.enabled && menu.active == Some(index);
        if active {
            p.fill_rounded(*rect, 5.0, Rgba(45, 212, 191, 46));
        }
        let color = if !row.enabled {
            Rgba(100, 116, 139, 255)
        } else if active {
            Rgba(248, 250, 252, 255)
        } else {
            Rgba(229, 231, 235, 255)
        };
        let secondary = if row.enabled {
            Rgba(148, 163, 184, 255)
        } else {
            color
        };
        let top = rect.top() + (rect.height() - menu.layout.line_height) / 2.0;
        p.text_buffer(&text.label, (rect.left() + 8.0, top), color.into());
        p.text_buffer(
            &text.shortcut,
            (rect.right() - 8.0 - text.shortcut_width, top),
            secondary.into(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::context_menu::Row;

    /// 【上下文菜单测试】【原版文字】返回七个文字动作和两处分隔线，重做禁用，其余动作可用，供真实缓冲与裁剪验证
    fn rows() -> Vec<Row> {
        crate::app::text_context_menu::ITEMS
            .iter()
            .map(|&(label, shortcut, command)| Row {
                label,
                shortcut,
                enabled: command != Some(crate::tools::text::session_input::Command::Redo),
            })
            .collect()
    }

    /// 【上下文菜单测试】【损坏一致】比例为一倍、分数和两倍时局部恢复后重绘与整帧一致，不累积透明背景，无参数、无返回值
    #[test]
    fn clipped_menu_redraw_matches_a_fresh_frame_at_fractional_scales() {
        for scale in [1.0, 1.2, 2.0] {
            let mut fonts = crate::fonts::bundled();
            let mut swash = cosmic_text::SwashCache::new();
            let mut menu = ContextMenu::new(rows(), &mut fonts, (20.0, 30.0), (400.0, 320.0), 0);
            let mut expected = Pixmap::new((400.0 * scale) as u32, (320.0 * scale) as u32).unwrap();
            expected.fill(Color::from_rgba8(30, 34, 38, 255));
            draw(
                &mut Painter::new(&mut expected)
                    .with_scale(scale)
                    .with_text(&mut fonts, &mut swash),
                &mut menu,
                None,
            );
            if let Some(directory) = std::env::var_os("MARK_SHOT_TEXT_MENU_ARTIFACTS") {
                std::fs::create_dir_all(&directory).unwrap();
                menu.pixmap
                    .as_ref()
                    .unwrap()
                    .save_png(std::path::Path::new(&directory).join(format!("menu-{scale}.png")))
                    .unwrap();
            }
            let mut actual = expected.clone();
            let dirty = Rect::from_xywh(35.0, 41.0, 96.0, 81.0).unwrap();
            let pixels = crate::renderer::scale_rect(&dirty, scale).unwrap();
            let (x, y, width, height) =
                crate::renderer::rect_bounds(&pixels, actual.width(), actual.height()).unwrap();
            let restored =
                Rect::from_xywh(x as f32, y as f32, width as f32, height as f32).unwrap();
            let mut paint = tiny_skia::Paint::default();
            paint.set_color_rgba8(30, 34, 38, 255);
            paint.anti_alias = false;
            for _ in 0..3 {
                actual.fill_rect(restored, &paint, tiny_skia::Transform::identity(), None);
                draw(
                    &mut Painter::new(&mut actual)
                        .with_scale(scale)
                        .with_text(&mut fonts, &mut swash),
                    &mut menu,
                    Some(&dirty),
                );
                let different = actual
                    .data()
                    .iter()
                    .zip(expected.data())
                    .filter(|(left, right)| left != right)
                    .count();
                assert_eq!(different, 0, "partial alpha composite differs at {scale}");
            }
        }
    }

    /// 【上下文菜单测试】【屏幕翻转】菜单先向锚点左上方翻转，两侧均放不下时贴边，超大菜单保留左上角，无参数、无返回值
    #[test]
    fn placement_flips_before_clamping_to_the_screen() {
        use crate::ui::context_menu::layout::position;
        assert_eq!(
            position((900.0, 600.0), (225.0, 229.0), (1067.0, 667.0)),
            (676.0, 372.0)
        );
        assert_eq!(
            position((100.0, 100.0), (225.0, 229.0), (300.0, 300.0)),
            (75.0, 71.0)
        );
        assert_eq!(
            position((100.0, 100.0), (225.0, 229.0), (120.0, 120.0)),
            (0.0, 0.0)
        );
    }
}
