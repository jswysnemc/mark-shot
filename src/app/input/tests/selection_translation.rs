//! 【框选翻译测试】【真实导出】验证冻结原图、显示切换和贴图交接使用独立来源

use super::*;
use crate::{
    features::{
        image_translation::{BoxRect, Token, TranslatedBlock, TranslatedLine, seed::Seed},
        selection_translation::{pixels::Region, session::Session},
    },
    ui::selection_translation::result_bar::Action,
};
use tiny_skia::Rect;

/// 【框选翻译测试】【结果样本】在 state 当前选区创建完整译文和已绘制像素，返回原图字节
fn translated(state: &mut EditorState) -> Vec<u8> {
    let region = Region::capture(state, state.selection.zone.unwrap()).unwrap();
    let original = region.original.data().to_vec();
    let mut session = Session::new(region);
    session.tokens.push(Token {
        text: "source".into(),
        rect: BoxRect::new(10.0, 10.0, 80.0, 20.0),
        line: 0,
        index: 0,
        confidence: 1.0,
    });
    let mut pixels = session.region.original.clone();
    pixels.fill(Color::from_rgba8(245, 240, 230, 255));
    assert!(session.complete(
        "English".into(),
        pixels,
        vec![TranslatedBlock {
            source: BoxRect::new(10.0, 10.0, 80.0, 20.0),
            plate: BoxRect::new(8.0, 8.0, 84.0, 24.0),
            background: crate::theme::Rgba(245, 240, 230, 255),
            foreground: crate::theme::Rgba(20, 24, 30, 255),
            font_size: 12.0,
            text: "complete translated paragraph".into(),
            lines: vec![TranslatedLine {
                text: "complete…".into(),
                rect: BoxRect::new(10.0, 10.0, 80.0, 20.0),
                baseline: 25.0
            }],
        }]
    ));
    state.selection_translation.session = Some(session);
    crate::app::selection_translation::apply(state, &mut 0);
    original
}

/// 【框选翻译测试】【显示与导出】原文和译文切换改变真实导出，退出逐字节恢复原始截图，不改变历史
#[test]
fn visible_translation_is_exported_and_exit_restores_original_capture() {
    let mut state = editor();
    state.selection.zone = Rect::from_xywh(100.0, 100.0, 200.0, 100.0);
    let before = state.captures[0].pixmap.data().to_vec();
    let original = translated(&mut state);
    let png = crate::app::render_final(&mut state).unwrap().0;
    assert_ne!(Pixmap::decode_png(&png).unwrap().data(), original);
    state
        .selection_translation
        .session
        .as_mut()
        .unwrap()
        .showing_translation = false;
    crate::app::selection_translation::apply(&mut state, &mut 0);
    assert_eq!(state.captures[0].pixmap.data(), before);
    state
        .selection_translation
        .session
        .as_mut()
        .unwrap()
        .showing_translation = true;
    crate::app::selection_translation::apply(&mut state, &mut 0);
    crate::app::selection_translation::exit(&mut state, &mut 0);
    assert_eq!(state.captures[0].pixmap.data(), before);
    assert!(state.undo_stack.is_empty());
    assert!(state.selection_translation.session.is_none());
}

/// 【框选翻译测试】【裁切交接】选区改变后贴图交接裁切原图并平移 token 和基线，交接不改变当前译文显示
#[test]
fn pin_receives_original_pixels_and_shifted_full_translation_after_selection_changes() {
    let mut state = editor();
    state.selection.zone = Rect::from_xywh(100.0, 100.0, 200.0, 100.0);
    translated(&mut state);
    state.selection.zone = Rect::from_xywh(105.0, 105.0, 100.0, 60.0);
    let visible = state.captures[0].pixmap.data().to_vec();
    let image = crate::app::selection_translation::handoff::prepare(&mut state).unwrap();
    let pixels = Pixmap::decode_png(&image.png).unwrap();
    assert_eq!(pixels.width(), 100);
    assert!(
        pixels
            .data()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [30, 34, 38, 255])
    );
    assert_eq!(
        image.seed.source_tokens[0].rect,
        BoxRect::new(5.0, 5.0, 80.0, 20.0)
    );
    assert_eq!(image.seed.blocks[0].lines[0].baseline, 20.0);
    assert_eq!(image.seed.blocks[0].text, "complete translated paragraph");
    assert!(image.seed.show_translation);
    assert_eq!(state.captures[0].pixmap.data(), visible);
    image.seed.validate().unwrap();
}

/// 【框选翻译测试】【浮条点击】右键不能切换原文，左键释放在其他按钮不触发，完整点击才恢复原图
#[test]
fn result_bar_requires_matching_left_press_and_release() {
    let mut state = editor();
    state.selection.zone = Rect::from_xywh(100.0, 100.0, 300.0, 180.0);
    translated(&mut state);
    let bar = state.selection_translation.bar.as_ref().unwrap();
    let original = bar
        .cells
        .iter()
        .find(|(a, _)| *a == Action::Original)
        .unwrap()
        .1;
    let point = (
        bar.rect.left() + original.left() + original.width() / 2.0,
        bar.rect.top() + original.top() + original.height() / 2.0,
    );
    pointer(&mut state, f64::from(point.0), f64::from(point.1), &mut 0);
    for pressed in [true, false] {
        super::super::handle_pointer_button(
            &mut state,
            crate::types::MouseButton::Right,
            pressed,
            &mut 0,
        );
    }
    assert!(
        state
            .selection_translation
            .session
            .as_ref()
            .unwrap()
            .showing_translation
    );
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Left, true, &mut 0);
    pointer(&mut state, 500.0, 400.0, &mut 0);
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Left, false, &mut 0);
    assert!(
        state
            .selection_translation
            .session
            .as_ref()
            .unwrap()
            .showing_translation
    );
    pointer(&mut state, f64::from(point.0), f64::from(point.1), &mut 0);
    for pressed in [true, false] {
        super::super::handle_pointer_button(
            &mut state,
            crate::types::MouseButton::Left,
            pressed,
            &mut 0,
        );
    }
    assert!(
        !state
            .selection_translation
            .session
            .as_ref()
            .unwrap()
            .showing_translation
    );
    assert!(state.pending.is_none());
    assert!(!state.input.mouse_down);
}

/// 【框选翻译测试】【透明预览】原图含透明通道且显示比例不足一倍时，译文预览与完整重绘一致，恢复不改变任意原始像素
#[test]
fn translucent_file_preview_matches_fresh_render_and_restores_exact_original() {
    let mut state = editor();
    let mut original = Pixmap::new(1200, 600).unwrap();
    original.fill(Color::from_rgba8(150, 50, 30, 128));
    let file = crate::editor::file_canvas::FileCanvas::new(
        original,
        Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap(),
    )
    .unwrap();
    state.selection.zone = Rect::from_xywh(
        file.viewport.left() + 20.25,
        file.viewport.top() + 10.5,
        150.4,
        60.7,
    );
    state.file_canvas = Some(file);
    let file = state.file_canvas.as_ref().unwrap();
    let transform = tiny_skia::Transform::from_scale(
        file.viewport.width() / file.original.width() as f32,
        file.viewport.height() / file.original.height() as f32,
    )
    .post_translate(file.viewport.left(), file.viewport.top());
    state.captures[0]
        .pixmap
        .fill(crate::theme::canvas::current().color());
    state.captures[0].pixmap.pixmap_mut().draw_pixmap(
        0,
        0,
        file.original.as_ref(),
        &tiny_skia::PixmapPaint {
            quality: tiny_skia::FilterQuality::Bilinear,
            ..Default::default()
        },
        transform,
        None,
    );
    let original = file.original.data().to_vec();
    let preview = state.captures[0].pixmap.data().to_vec();
    translated(&mut state);
    let mut expected = Pixmap::new(640, 480).unwrap();
    expected.fill(crate::theme::canvas::current().color());
    expected.draw_pixmap(
        0,
        0,
        state.file_canvas.as_ref().unwrap().original.as_ref(),
        &tiny_skia::PixmapPaint {
            quality: tiny_skia::FilterQuality::Bilinear,
            ..Default::default()
        },
        transform,
        None,
    );
    assert_eq!(state.captures[0].pixmap.data(), expected.data());
    crate::app::selection_translation::exit(&mut state, &mut 0);
    assert_eq!(
        state.file_canvas.as_ref().unwrap().original.data(),
        original
    );
    assert_eq!(state.captures[0].pixmap.data(), preview);
}

/// 【框选翻译测试】【缓存语义】返回同语言缓存无需任务，空译文或未知目标保留完整显示
#[test]
fn target_cache_preserves_current_display_on_missing_or_empty_results() {
    let mut state = editor();
    state.selection.zone = Rect::from_xywh(100.0, 100.0, 200.0, 100.0);
    translated(&mut state);
    let session = state.selection_translation.session.as_mut().unwrap();
    let seed = Seed {
        source_tokens: session.tokens.clone(),
        blocks: session.current().unwrap().blocks.clone(),
        show_translation: true,
    };
    assert!(!session.cached("Japanese"));
    assert!(!session.complete(
        "Japanese".into(),
        session.region.original.clone(),
        Vec::new()
    ));
    assert_eq!(session.language, "English");
    assert!(session.cached("English"));
    assert_eq!(seed.blocks, session.current().unwrap().blocks);
    assert_eq!(session.translations.len(), 1);
    assert!(state.selection_translation.poll_interval().is_none());
}

/// 【框选翻译测试】【输出比例】跨输出选区缩到低比例输出后，贴图交接同步缩放字号和坐标，元数据仍合法
#[test]
fn handoff_rescales_seed_when_the_remaining_selection_uses_a_lower_scale_output() {
    let mut state = editor();
    let mut image = Pixmap::new(640, 480).unwrap();
    image.fill(Color::from_rgba8(30, 34, 38, 255));
    state.captures.push(Capture {
        pixmap: image.into(),
        scale: 2.0,
    });
    state.placements.push(Placement {
        position: (640, 0),
        size: (320, 240),
        scale: 2.0,
    });
    state.selection.zone = Rect::from_xywh(600.0, 100.0, 100.0, 100.0);
    translated(&mut state);
    state.selection.zone = Rect::from_xywh(600.0, 100.0, 40.0, 100.0);
    let image = crate::app::selection_translation::handoff::prepare(&mut state).unwrap();
    assert_eq!(image.seed.blocks[0].font_size, 6.0);
    assert_eq!(
        image.seed.source_tokens[0].rect,
        BoxRect::new(5.0, 5.0, 35.0, 10.0)
    );
    image.seed.validate().unwrap();
}
