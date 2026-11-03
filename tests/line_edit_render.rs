//! 【字段像素测试】【裁切一致】原生候选字段不覆盖相邻控件，分数缩放下局部重绘等于整帧

use cosmic_text::{SwashCache, Weight};
use mark_shot::{
    renderer::Painter,
    types::text_input::{Edit, Preedit},
    ui::text_field::{LineEditState, validation::Rule},
};
use tiny_skia::{Color, Pixmap, Rect};

/// 【字段像素测试】【原生字段】按 candidate 返回带普通选区或候选的长字段，正文与候选独立存储
fn field(candidate: bool) -> LineEditState {
    let mut field = LineEditState::new("alpha beta gamma delta".into());
    field.cursor = 7;
    field.selection_anchor = Some(3);
    if candidate {
        field.input_method(
            Edit {
                preedit: Preedit {
                    text: "你𠀀 ni".into(),
                    cursor: Some((10, 10)),
                },
                ..Default::default()
            },
            Rule::Text,
        );
    }
    field
}

/// 【字段像素测试】【真实绘制】按 field、scale、origin 与 dirty 绘制字段，返回实际像素；裁切使用生产画布方法
fn render(field: &LineEditState, scale: f32, origin: (f32, f32), dirty: Option<Rect>) -> Pixmap {
    let mut image = Pixmap::new(280, 120).unwrap();
    image.fill(Color::from_rgba8(30, 34, 38, 255));
    let mut fonts = mark_shot::fonts::bundled();
    let mut cache = SwashCache::new();
    let rect = Rect::from_xywh(34.25, 24.5, 96.0, 28.0).unwrap();
    Painter::new(&mut image)
        .with_text(&mut fonts, &mut cache)
        .with_scale(scale)
        .with_origin(origin)
        .clipped(dirty, |p| {
            p.line_edit(
                rect,
                &field.text,
                Some(field),
                16.0,
                Color::from_rgba8(229, 231, 235, 255),
                Weight::NORMAL,
            );
        });
    image
}

/// 【字段像素测试】【越界保护】候选、下划线、光标和选择只出现在字段缓冲，覆盖四种比例与偏移原点，无参数、无返回值
#[test]
fn candidate_and_selection_are_clipped_to_the_field_at_fractional_scales() {
    for scale in [0.75, 1.0, 1.2, 2.0] {
        for origin in [(0.0, 0.0), (13.25, 7.125)] {
            for candidate in [false, true] {
                let image = render(&field(candidate), scale, origin, None);
                let baseline = Color::from_rgba8(30, 34, 38, 255)
                    .premultiply()
                    .to_color_u8();
                let x = ((34.25 - origin.0) * scale).round() as i32;
                let y = ((24.5 - origin.1) * scale).round() as i32;
                let right = x + (96.0 * scale).ceil() as i32;
                let bottom = y + (28.0 * scale).ceil() as i32;
                let mut painted = 0;
                for row in 0..image.height() {
                    for column in 0..image.width() {
                        let pixel = image.pixel(column, row).unwrap();
                        if pixel == baseline {
                            continue;
                        }
                        painted += 1;
                        assert!(
                            (x..right).contains(&(column as i32))
                                && (y..bottom).contains(&(row as i32)),
                            "candidate={candidate}, scale={scale}, origin={origin:?}, pixel={column},{row}"
                        );
                    }
                }
                assert!(painted > 20);
            }
        }
    }
}

/// 【字段像素测试】【局部对照】窄损坏范围内逐字节等于整帧，范围外保持背景，覆盖候选与选择，无参数、无返回值
#[test]
fn partial_field_damage_matches_full_frame_pixel_for_pixel() {
    for scale in [0.75, 1.0, 1.2, 2.0] {
        for origin in [(0.0, 0.0), (13.25, 7.125)] {
            for candidate in [false, true] {
                let field = field(candidate);
                let dirty = Rect::from_xywh(30.0, 12.0, 25.0, 50.0).unwrap();
                let full = render(&field, scale, origin, None);
                let local = render(&field, scale, origin, Some(dirty));
                let left = (dirty.left() * scale).floor() as u32;
                let top = (dirty.top() * scale).floor() as u32;
                let right = (dirty.right() * scale).ceil() as u32;
                let bottom = (dirty.bottom() * scale).ceil() as u32;
                for y in top..bottom {
                    for x in left..right {
                        assert_eq!(
                            local.pixel(x, y),
                            full.pixel(x, y),
                            "candidate={candidate}, scale={scale}, origin={origin:?}, pixel={x},{y}"
                        );
                    }
                }
            }
        }
    }
}
