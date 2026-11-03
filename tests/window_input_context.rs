//! 【窗口输入法测试】【显示上下文】验证实际编辑器的有界正文、隐藏光标和分数缩放裁切

use cosmic_text::{FontSystem, SwashCache};
use mark_shot::{
    types::text_input::{ChangeCause, Edit, Focus, Preedit, Snapshot},
    ui::text_editor::{InputArea, TextEditor},
};
use tiny_skia::{Pixmap, Rect};

/// 【窗口输入法测试】【快照取得】用 fonts 获取 editor 在 rect 内的真实协议快照，previous 用于静止去重，返回变化或空值
fn snapshot(
    editor: &mut TextEditor,
    fonts: &mut FontSystem,
    rect: Rect,
    previous: Option<&Snapshot>,
) -> Option<Snapshot> {
    editor.input_snapshot(
        fonts,
        InputArea { rect, offset: 0.0 },
        Focus {
            id: 100,
            monitor_idx: 0,
        },
        200,
        previous,
        ChangeCause::Other,
    )
}

/// 【窗口输入法测试】【字节边界】长文档与跨行选择仍发送合法 UTF-8，正文不超过四千字节，无参数、无返回值
#[test]
fn surrounding_context_is_bounded_and_preserves_document_offsets() {
    let text = format!(
        "{}\r\n{}\n{}",
        "你𠮷a".repeat(1000),
        "好b".repeat(1200),
        "tail".repeat(1500)
    );
    let boundaries: Vec<_> = text.char_indices().map(|(index, _)| index).collect();
    let mut fonts = mark_shot::fonts::bundled();
    let mut editor = TextEditor::new(&text, &mut fonts);
    let rect = Rect::from_xywh(11.0, 19.0, 320.0, 100.0).unwrap();
    for index in (0..boundaries.len()).step_by(127) {
        let cursor = boundaries[index];
        let anchor = boundaries[(index + 31).min(boundaries.len() - 1)];
        assert!(editor.set_byte_selection(&mut fonts, cursor, anchor));
        let value = snapshot(&mut editor, &mut fonts, rect, None).unwrap();
        let surrounding = value.surrounding.unwrap();
        assert!(surrounding.text.len() <= 4000);
        let start = surrounding.document_start;
        assert_eq!(
            text.get(start..start + surrounding.text.len()),
            Some(surrounding.text.as_str())
        );
        assert_eq!(start + surrounding.cursor as usize, cursor);
        assert_eq!(start + surrounding.anchor as usize, anchor);
        assert!(
            surrounding
                .text
                .is_char_boundary(surrounding.cursor as usize)
        );
        assert!(
            surrounding
                .text
                .is_char_boundary(surrounding.anchor as usize)
        );
    }
}

/// 【窗口输入法测试】【上下文拒绝】超长选择和空字符不能进入协议周边正文，无参数、无返回值
#[test]
fn oversized_selection_and_nul_context_are_omitted() {
    let mut fonts = mark_shot::fonts::bundled();
    let rect = Rect::from_xywh(0.0, 0.0, 200.0, 100.0).unwrap();
    let mut editor = TextEditor::new(&"a".repeat(5000), &mut fonts);
    editor.set_byte_selection(&mut fonts, 5000, 0);
    assert!(
        snapshot(&mut editor, &mut fonts, rect, None)
            .unwrap()
            .surrounding
            .is_none()
    );
    let mut editor = TextEditor::new("before\0after", &mut fonts);
    assert!(
        snapshot(&mut editor, &mut fonts, rect, None)
            .unwrap()
            .surrounding
            .is_none()
    );
}

/// 【窗口输入法测试】【静止去重】同一编辑器重复同步不产生快照，候选只改变显示光标不改变正文上下文，无参数、无返回值
#[test]
fn unchanged_snapshots_are_suppressed_before_and_during_preedit() {
    let mut fonts = mark_shot::fonts::bundled();
    let rect = Rect::from_xywh(7.0, 9.0, 240.0, 70.0).unwrap();
    let mut editor = TextEditor::new("base", &mut fonts);
    editor.set_byte_selection(&mut fonts, 4, 4);
    let before = snapshot(&mut editor, &mut fonts, rect, None).unwrap();
    for _ in 0..20 {
        assert!(snapshot(&mut editor, &mut fonts, rect, Some(&before)).is_none());
    }
    editor.input_method(
        &mut fonts,
        Edit {
            preedit: Preedit {
                text: "ni".into(),
                cursor: Some((2, 2)),
            },
            ..Edit::default()
        },
    );
    let after = snapshot(&mut editor, &mut fonts, rect, Some(&before)).unwrap();
    assert_eq!(after.surrounding, before.surrounding);
    assert!(after.cursor_rect.unwrap().0 > before.cursor_rect.unwrap().0);
    for _ in 0..20 {
        assert!(snapshot(&mut editor, &mut fonts, rect, Some(&after)).is_none());
    }
}

/// 【窗口输入法测试】【实际绘制】将 editor 以 scale 绘制到独立画布，rect 控制逻辑裁切、focused 控制光标，返回原始像素
fn draw(
    editor: &mut TextEditor,
    fonts: &mut FontSystem,
    rect: Rect,
    scale: f32,
    focused: bool,
) -> Pixmap {
    let mut canvas = Pixmap::new(220, 150).unwrap();
    editor.draw(
        fonts,
        &mut SwashCache::new(),
        &mut canvas,
        rect,
        scale,
        focused,
    );
    canvas
}

/// 【窗口输入法测试】【隐藏光标】输入法隐藏光标时聚焦画面与无焦点画面完全相同，候选字形与下划线仍存在，无参数、无返回值
#[test]
fn hidden_preedit_cursor_is_not_drawn_at_any_supported_scale() {
    let mut fonts = mark_shot::fonts::bundled();
    let rect = Rect::from_xywh(13.0, 17.0, 74.0, 43.0).unwrap();
    let mut editor = TextEditor::new("base", &mut fonts);
    editor.set_byte_selection(&mut fonts, 4, 4);
    editor.input_method(
        &mut fonts,
        Edit {
            preedit: Preedit {
                text: "ni".into(),
                cursor: None,
            },
            ..Edit::default()
        },
    );
    for scale in [0.75, 1.0, 1.2, 2.0] {
        let focused = draw(&mut editor, &mut fonts, rect, scale, true);
        let unfocused = draw(&mut editor, &mut fonts, rect, scale, false);
        assert_eq!(focused.data(), unfocused.data(), "scale {scale}");
        assert!(focused.pixels().iter().any(|pixel| pixel.alpha() != 0));
        for (index, pixel) in focused.pixels().iter().enumerate() {
            if pixel.alpha() == 0 {
                continue;
            }
            let x = (index % focused.width() as usize) as f32;
            let y = (index / focused.width() as usize) as f32;
            assert!(x >= (rect.left() * scale).round() && x < (rect.right() * scale).ceil());
            assert!(y >= (rect.top() * scale).round() && y < (rect.bottom() * scale).ceil());
        }
    }
}
