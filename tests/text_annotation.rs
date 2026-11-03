//! 【文字测试】【排版绘制】验证原版点字号、字体缓存、背景边界与透明度，实际原版像素差异单独输出

use std::collections::HashMap;

use cosmic_text::{Action, Edit, Editor, FontSystem, Motion, Selection, SwashCache};
use mark_shot::{
    features::text::{TextAppearance, geometry},
    tools::text::{ensure_text_editor, render_text_annotation, update_text_bbox_inline},
    types::{Annotation, AnnotationShape},
};
use tiny_skia::{Color, Pixmap, Rect};

/// 【文字测试】【标注构造】按 text 创建二十点半粗体文字，使用内置 Inter 使普通测试不依赖系统字体；返回未排版标注
fn annotation(text: &str) -> Annotation {
    Annotation {
        id: 7,
        shape: AnnotationShape::Text {
            start: (16.0, 16.0),
            content: text.into(),
            font_size: 20.0,
            bold: true,
            italic: false,
            outline: None,
            appearance: TextAppearance {
                family: "Inter".into(),
                ..Default::default()
            },
        },
        color: Color::from_rgba8(255, 77, 77, 255),
        shadow_color: Color::TRANSPARENT,
        stroke_width: 0.0,
        bbox: Rect::from_xywh(16.0, 16.0, 1.0, 1.0).unwrap(),
    }
}

/// 【文字测试】【排版环境】根据 ann 返回已同步边界的编辑器与字体系统；不连接显示服务器
fn layout(ann: &mut Annotation) -> (Editor<'static>, FontSystem) {
    let mut fonts = mark_shot::fonts::bundled();
    let mut editors = HashMap::new();
    let editor = ensure_text_editor(ann, &mut editors, &mut fonts);
    update_text_bbox_inline(ann, editor, &mut fonts);
    (editors.remove(&ann.id).unwrap(), fonts)
}

/// 【文字测试】【透明绘制】将 ann、editor 绘入 size 大小的透明图；fonts 提供字体，scale、offset 映射坐标，editing 控制选区，返回像素图
fn draw(
    ann: &Annotation,
    editor: &mut Editor<'static>,
    fonts: &mut FontSystem,
    size: (u32, u32),
    scale: f32,
    offset: (f32, f32),
    editing: bool,
) -> Pixmap {
    let mut image = Pixmap::new(size.0, size.1).unwrap();
    render_text_annotation(
        ann,
        editor,
        fonts,
        &mut SwashCache::new(),
        &mut image.as_mut(),
        offset,
        scale,
        editing,
    );
    image
}

/// 【文字测试】【像素字号】点字号先取整再转换并取整像素，覆盖默认、上下限及缩小显示
#[test]
fn point_sizes_match_the_qt_integer_pixel_conversion() {
    assert_eq!(geometry::points(f32::NAN), 20.0);
    assert_eq!(geometry::points(7.0), 8.0);
    assert_eq!(geometry::points(300.7), 300.0);
    assert_eq!(geometry::points(20.5), 21.0);
    for (points, scale, expected) in [
        (20.0, 1.0, 27.0),
        (20.0, 0.5, 13.0),
        (20.0, 0.75, 20.0),
        (20.0, 1.25, 33.0),
        (8.0, 1.0, 11.0),
        (300.0, 1.0, 400.0),
    ] {
        assert_eq!(geometry::pixels(points, scale), expected);
    }
}

/// 【文字测试】【背景内边距】透明背景与不透明背景共用左右六、上下四个原图像素，边界向上取整
#[test]
fn transparent_background_keeps_symmetric_padding() {
    let mut ann = annotation("Mark Shot");
    let (editor, _) = layout(&mut ann);
    let (width, height) = editor.with_buffer(|buffer| {
        let run = buffer.layout_runs().next().unwrap();
        (run.line_w, run.line_height)
    });
    assert_eq!(ann.bbox.width(), (width + 12.0).ceil());
    assert_eq!(ann.bbox.height(), (height + 8.0).ceil());
    assert_eq!(ann.bbox.left(), 16.0);
    assert_eq!(geometry::padding(0.5), (3.0, 2.0));
    assert_eq!(geometry::bounds((0.0, 0.0), 10.1, 20.2, 0.5).width(), 17.0);
}

/// 【文字测试】【空行度量】空文字保留一行高度，多行与末尾空行均进入文字框高度
#[test]
fn empty_and_trailing_lines_contribute_to_bounds() {
    let mut empty = annotation("");
    let (editor, _) = layout(&mut empty);
    let height = editor.with_buffer(|buffer| buffer.metrics().line_height);
    assert_eq!(empty.bbox.height(), (height + 8.0).ceil());
    let mut multiline = annotation("a\nb\n");
    let (editor, _) = layout(&mut multiline);
    assert_eq!(editor.with_buffer(|buffer| buffer.lines.len()), 3);
    assert_eq!(multiline.bbox.height(), (height * 3.0 + 8.0).ceil());
}

/// 【文字测试】【字体缓存】内置字体无需系统安装，重复同步保留光标与选择；字号变化时同步新缓存与边界
#[test]
fn cached_layout_keeps_selection_but_updates_changed_properties() {
    let mut ann = annotation("Mark Shot");
    let (mut editor, mut fonts) = layout(&mut ann);
    editor.set_selection(Selection::Normal(cosmic_text::Cursor::new(0, 0)));
    editor.action(&mut fonts, Action::Motion(Motion::BufferEnd));
    let selection = editor.selection_bounds();
    let cursor = editor.cursor();
    let old = ann.bbox;
    update_text_bbox_inline(&mut ann, &mut editor, &mut fonts);
    assert_eq!(editor.cursor(), cursor);
    assert_eq!(editor.selection_bounds(), selection);
    editor.with_buffer(|buffer| {
        assert_eq!(buffer.metrics().font_size, 27.0);
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                assert_eq!(
                    fonts.db().face(glyph.font_id).unwrap().families[0].0,
                    "Inter"
                );
            }
        }
    });
    if let AnnotationShape::Text { font_size, .. } = &mut ann.shape {
        *font_size = 40.0;
    }
    update_text_bbox_inline(&mut ann, &mut editor, &mut fonts);
    assert_eq!(
        editor.with_buffer(|buffer| buffer.metrics().font_size),
        53.0
    );
    assert!(ann.bbox.width() > old.width());
}

/// 【文字测试】【字体样式】常规、半粗体、斜体属性各自更新排版，不因家族相同而使用旧字重
#[test]
fn font_attributes_follow_regular_demibold_and_italic_changes() {
    let mut ann = annotation("Styles");
    let (mut editor, mut fonts) = layout(&mut ann);
    for (bold, italic) in [(false, false), (true, false), (true, true), (false, true)] {
        if let AnnotationShape::Text {
            bold: b, italic: i, ..
        } = &mut ann.shape
        {
            *b = bold;
            *i = italic;
        }
        update_text_bbox_inline(&mut ann, &mut editor, &mut fonts);
        editor.with_buffer(|buffer| {
            let attrs = buffer.lines[0].attrs_list().defaults();
            assert_eq!(attrs.weight.0, if bold { 600 } else { 400 });
            assert_eq!(
                attrs.style,
                if italic {
                    cosmic_text::Style::Italic
                } else {
                    cosmic_text::Style::Normal
                }
            );
            for run in buffer.layout_runs() {
                for glyph in run.glyphs {
                    let face = fonts.db().face(glyph.font_id).unwrap();
                    assert_eq!(face.style, attrs.style);
                    if bold {
                        assert!([600, 700].contains(&face.weight.0));
                    } else {
                        assert_eq!(face.weight.0, 400);
                    }
                }
            }
        });
    }
}

/// 【文字测试】【正文透明度】正文透明度只与字形覆盖率相乘一次，完全覆盖像素为指定 alpha
#[test]
fn text_alpha_is_applied_exactly_once() {
    for alpha in [0, 64, 128, 255] {
        let mut ann = annotation("MMMM");
        ann.color = Color::from_rgba8(200, 100, 50, alpha);
        let (mut editor, mut fonts) = layout(&mut ann);
        let image = draw(
            &ann,
            &mut editor,
            &mut fonts,
            (240, 100),
            1.0,
            (0.0, 0.0),
            false,
        );
        assert_eq!(
            image.pixels().iter().map(|pixel| pixel.alpha()).max(),
            Some(alpha)
        );
    }
}

/// 【文字测试】【独立背景】背景不继承正文 alpha，正文完全透明时仍绘制背景且不改变边界
#[test]
fn background_alpha_is_independent_from_text_alpha() {
    let mut ann = annotation("Background");
    if let AnnotationShape::Text { appearance, .. } = &mut ann.shape {
        appearance.background = Color::from_rgba8(30, 60, 90, 96);
    }
    let (mut editor, mut fonts) = layout(&mut ann);
    let bbox = ann.bbox;
    for alpha in [0, 64, 255] {
        ann.color = Color::from_rgba8(200, 100, 50, alpha);
        let image = draw(
            &ann,
            &mut editor,
            &mut fonts,
            (240, 100),
            1.0,
            (0.0, 0.0),
            false,
        );
        assert_eq!(image.pixel(20, 24).unwrap().alpha(), 96);
        assert_eq!(image.pixel(0, 0).unwrap().alpha(), 0);
        assert_eq!(ann.bbox, bbox);
    }
}

/// 【文字测试】【选择导出】退出编辑后旧字符选择不能改变最终正文颜色，也不能绘制光标与选择背景
#[test]
fn inactive_selection_does_not_leak_into_export_pixels() {
    let mut ann = annotation("Selection");
    let (mut editor, mut fonts) = layout(&mut ann);
    let expected = draw(
        &ann,
        &mut editor,
        &mut fonts,
        (240, 100),
        1.0,
        (0.0, 0.0),
        false,
    );
    editor.set_selection(Selection::Normal(cosmic_text::Cursor::new(0, 0)));
    editor.action(&mut fonts, Action::Motion(Motion::BufferEnd));
    let selected = draw(
        &ann,
        &mut editor,
        &mut fonts,
        (240, 100),
        1.0,
        (0.0, 0.0),
        false,
    );
    assert_eq!(expected.data(), selected.data());
    let active = draw(
        &ann,
        &mut editor,
        &mut fonts,
        (240, 100),
        1.0,
        (0.0, 0.0),
        true,
    );
    assert_ne!(expected.data(), active.data());
}

/// 【文字测试】【颜色职责】正文恰好使用选择背景色时也只画字形，不能把字形像素扩成整行选择矩形
#[test]
fn selection_colored_glyphs_are_not_treated_as_highlight_rectangles() {
    let mut ann = annotation("MMMM");
    ann.color = mark_shot::theme::color::text_selection().color();
    let (mut editor, mut fonts) = layout(&mut ann);
    let expected = draw(
        &ann,
        &mut editor,
        &mut fonts,
        (240, 100),
        1.0,
        (0.0, 0.0),
        false,
    );
    let editing = draw(
        &ann,
        &mut editor,
        &mut fonts,
        (240, 100),
        1.0,
        (0.0, 0.0),
        true,
    );
    // 1. 【文字测试】【光标排除】光标只有二十二像素处的一列，其右侧必须与未编辑字形逐像素一致
    for y in 0..100 {
        for x in 23..240 {
            assert_eq!(
                expected.pixel(x, y),
                editing.pixel(x, y),
                "pixel at {x},{y}"
            );
        }
    }
}

/// 【文字测试】【坐标映射】原图缩放只改变预览比例与换行宽度，不改点字号；逆向映射恢复相同导出像素
#[test]
fn scaling_preserves_points_and_round_trip_export_pixels() {
    let mut ann = annotation("Original pixels");
    let (mut editor, mut fonts) = layout(&mut ann);
    let expected = draw(
        &ann,
        &mut editor,
        &mut fonts,
        (280, 100),
        1.0,
        (0.0, 0.0),
        false,
    );
    let mut restored = ann.scaled(0.5).scaled(2.0);
    assert_eq!(restored.shape, ann.shape);
    let (mut restored_editor, mut restored_fonts) = layout(&mut restored);
    let actual = draw(
        &restored,
        &mut restored_editor,
        &mut restored_fonts,
        (280, 100),
        1.0,
        (0.0, 0.0),
        false,
    );
    assert_eq!(expected.data(), actual.data());
    let shifted = draw(
        &ann,
        &mut editor,
        &mut fonts,
        (280, 100),
        1.0,
        (10.0, 10.0),
        false,
    );
    for y in 10..100 {
        for x in 10..280 {
            assert_eq!(expected.pixel(x, y), shifted.pixel(x - 10, y - 10));
        }
    }
}

/// 【文字测试】【框宽换行】换行宽度扣除左右内边距，缩放后一起映射，放宽框后恢复自然宽度
#[test]
fn wrapping_uses_content_width_and_scales_with_the_background() {
    let mut ann = annotation("one two three four five six");
    let (mut editor, mut fonts) = layout(&mut ann);
    let unwrapped = ann.bbox;
    if let AnnotationShape::Text { appearance, .. } = &mut ann.shape {
        appearance.wrap_width = Some(100.0);
    }
    update_text_bbox_inline(&mut ann, &mut editor, &mut fonts);
    assert_eq!(editor.with_buffer(|buffer| buffer.size().0), Some(88.0));
    assert!(editor.with_buffer(|buffer| buffer.layout_runs().count()) > 1);
    assert!(ann.bbox.height() > unwrapped.height());
    assert!(ann.bbox.width() <= 100.0);
    if let AnnotationShape::Text {
        appearance,
        font_size,
        ..
    } = ann.scaled(0.5).shape
    {
        assert_eq!(appearance.wrap_width, Some(50.0));
        assert_eq!(appearance.size_scale, 0.5);
        assert_eq!(font_size, 20.0);
    }
    if let AnnotationShape::Text { appearance, .. } = &mut ann.shape {
        appearance.wrap_width = None;
    }
    update_text_bbox_inline(&mut ann, &mut editor, &mut fonts);
    assert_eq!(ann.bbox, unwrapped);
}

/// 【文字测试】【过窄框】可用宽度不足十六个原图像素时不启用换行，缩小预览也使用相同判断
#[test]
fn too_narrow_boxes_disable_wrapping_at_every_scale() {
    for scale in [0.5, 0.75, 1.0, 1.25] {
        let mut ann = annotation("Mark Shot");
        if let AnnotationShape::Text { appearance, .. } = &mut ann.shape {
            appearance.size_scale = scale;
            appearance.wrap_width = Some(20.0 * scale);
        }
        let (editor, _) = layout(&mut ann);
        assert_eq!(
            editor.with_buffer(|buffer| buffer.wrap()),
            cosmic_text::Wrap::None
        );
        assert_eq!(editor.with_buffer(|buffer| buffer.layout_runs().count()), 1);
    }
}

/// 【文字测试】【重复匹配】反复取得同一家族四种样式不重复加载字体文件，界面默认家族仍为内置 Inter
#[test]
fn repeated_font_matching_does_not_duplicate_faces_or_change_ui_family() {
    let mut fonts = mark_shot::fonts::bundled();
    for (bold, italic) in [(false, false), (true, false), (true, true), (false, true)] {
        let family = mark_shot::fonts::text_faces::family(&mut fonts, "Sans Serif", bold, italic);
        let count = fonts.db().len();
        for _ in 0..4 {
            assert_eq!(
                mark_shot::fonts::text_faces::family(&mut fonts, "Sans Serif", bold, italic),
                family
            );
            assert_eq!(fonts.db().len(), count);
        }
    }
    let id = fonts
        .db()
        .query(&cosmic_text::fontdb::Query {
            families: &[cosmic_text::fontdb::Family::SansSerif],
            ..Default::default()
        })
        .unwrap();
    assert_eq!(fonts.db().face(id).unwrap().families[0].0, "Inter");
}

/// 【文字测试】【原版证据】从 MARK_SHOT_TEXT_CASES 读取 JSON 场景，写入 MARK_SHOT_TEXT_ARTIFACTS；普通测试不写文件
#[test]
#[ignore = "requires explicit reference case and artifact directories"]
fn render_reference_fixture() {
    let cases = std::path::PathBuf::from(std::env::var("MARK_SHOT_TEXT_CASES").unwrap());
    let output = std::path::PathBuf::from(std::env::var("MARK_SHOT_TEXT_ARTIFACTS").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    for path in std::fs::read_dir(cases)
        .unwrap()
        .map(|entry| entry.unwrap().path())
    {
        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }
        let config: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let scale = config["scale"].as_f64().unwrap_or(1.0) as f32;
        let mut ann = annotation(config["text"].as_str().unwrap_or("Mark Shot"));
        ann.color = mark_shot::ui::color_popover::parse_hex_color(
            config["color"].as_str().unwrap_or("FF4D4D"),
        )
        .unwrap();
        if let AnnotationShape::Text {
            start,
            appearance,
            font_size,
            bold,
            italic,
            ..
        } = &mut ann.shape
        {
            *start = (16.0 * scale, 16.0 * scale);
            *font_size = config["points"].as_f64().unwrap_or(20.0) as f32;
            *bold = config["bold"].as_bool().unwrap_or(true);
            *italic = config["italic"].as_bool().unwrap_or(false);
            appearance.family = config["family"].as_str().unwrap_or("Sans Serif").into();
            appearance.size_scale = scale;
            appearance.wrap_width = config["wrap_width"]
                .as_f64()
                .map(|width| width as f32 * scale);
            appearance.background = mark_shot::ui::color_popover::parse_hex_color(
                config["background"].as_str().unwrap_or("00000000"),
            )
            .unwrap();
        }
        let mut fonts = mark_shot::fonts::bundled();
        mark_shot::fonts::add_system(&mut fonts, mark_shot::fonts::system());
        let mut editors = HashMap::new();
        let editor = ensure_text_editor(&ann, &mut editors, &mut fonts);
        update_text_bbox_inline(&mut ann, editor, &mut fonts);
        let mut editor = editors.remove(&ann.id).unwrap();
        let size = (
            (ann.bbox.right() + 16.0 * scale).ceil() as u32,
            (ann.bbox.bottom() + 16.0 * scale).ceil() as u32,
        );
        let image = draw(&ann, &mut editor, &mut fonts, size, 1.0, (0.0, 0.0), false);
        let name = path.file_stem().unwrap();
        image
            .save_png(output.join(name).with_extension("png"))
            .unwrap();
        let metrics = editor.with_buffer(|buffer| {
            let runs = buffer.layout_runs().collect::<Vec<_>>();
            let face = runs.iter().flat_map(|run| run.glyphs.iter()).next().and_then(|glyph| fonts.db().face(glyph.font_id));
            serde_json::json!({
                "bounds": [ann.bbox.left(), ann.bbox.top(), ann.bbox.width(), ann.bbox.height()],
                "font_pixels": buffer.metrics().font_size, "line_spacing": buffer.metrics().line_height,
                "font_family": face.map(|face| &face.families[0].0),
                "font_weight": face.map(|face| face.weight.0),
                "advance": runs.iter().map(|run| run.line_w).fold(0.0_f32, f32::max),
                "lines": runs.len(),
                "runs": runs.iter().map(|run| serde_json::json!({
                    "width": run.line_w,
                    "text": run.glyphs.iter().map(|glyph| glyph.start).min().zip(run.glyphs.iter().map(|glyph| glyph.end).max()).map(|(first,last)| &buffer.lines[run.line_i].text()[first..last]),
                    "families": run.glyphs.iter().map(|glyph| &fonts.db().face(glyph.font_id).unwrap().families[0].0).collect::<std::collections::BTreeSet<_>>(),
                })).collect::<Vec<_>>(),
            })
        });
        std::fs::write(
            output.join(name).with_extension("json"),
            serde_json::to_vec_pretty(&metrics).unwrap(),
        )
        .unwrap();
    }
}
