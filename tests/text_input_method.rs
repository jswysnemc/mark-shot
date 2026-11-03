//! 【输入法测试】【协议边界】核验 UTF-8 位置、有限上下文和原子交付，原版样本不代替生产接入验证

use cosmic_text::Cursor;
use mark_shot::{
    features::text::input_method::{positions, surrounding, transaction::Pending},
    types::text_input::{Batch, Preedit},
};

/// 【输入法测试】【多行往返】每个码点边界与行光标一致，组合附加符允许独立字节位置，无参数、无返回值
#[test]
fn every_utf8_boundary_round_trips_through_editor_positions() {
    let text = "A\u{e9}\u{754c}\na\u{301}\n";
    for offset in (0..=text.len()).filter(|offset| text.is_char_boundary(*offset)) {
        let cursor = positions::editor_cursor(text, offset).unwrap();
        assert_eq!(positions::document_offset(text, cursor), Some(offset));
    }
    assert_eq!(
        positions::editor_cursor(text, text.len()),
        Some(Cursor::new(2, 0))
    );
    assert_eq!(positions::document_offset(text, Cursor::new(1, 1)), Some(8));
}

/// 【输入法测试】【非法光标】越界行、码点中间字节和极端索引均返回空值，不能发生算术溢出，无参数、无返回值
#[test]
fn invalid_and_extreme_editor_positions_are_rejected_without_overflow() {
    let text = "x\n\u{754c}";
    assert_eq!(
        positions::document_offset(text, Cursor::new(1, usize::MAX)),
        None
    );
    assert_eq!(positions::document_offset(text, Cursor::new(8, 0)), None);
    assert_eq!(positions::document_offset(text, Cursor::new(1, 1)), None);
    assert_eq!(positions::editor_cursor(text, 3), None);
    assert_eq!(positions::editor_cursor(text, usize::MAX), None);
}

/// 【输入法测试】【字节删除】相对删除保留真实 UTF-8 长度，切开码点、越界或过长请求整体拒绝，无参数、无返回值
#[test]
fn surrounding_deletion_uses_bytes_and_rejects_split_codepoints() {
    let text = "A\u{e9}\u{754c}Z";
    assert_eq!(
        positions::deletion_range(text, 6, 5, 1),
        Some((Cursor::new(0, 1), Cursor::new(0, 7)))
    );
    assert_eq!(positions::deletion_range(text, 6, 4, 0), None);
    assert_eq!(positions::deletion_range(text, 2, 1, 1), None);
    assert_eq!(positions::deletion_range(text, 0, 1, 0), None);
    assert_eq!(positions::deletion_range(text, 6, u32::MAX, 0), None);
    assert_eq!(positions::deletion_range(text, 6, 0, u32::MAX), None);
}

/// 【输入法测试】【选区完整】长多字节文档保留正反两个方向的完整选区，上下文不超过四千字节，无参数、无返回值
#[test]
fn bounded_context_preserves_the_complete_bidirectional_selection() {
    let text = "\u{754c}\u{e9}".repeat(1800);
    for (cursor, anchor) in [(3000, 4000), (4000, 3000), (9000, 9000), (0, 0)] {
        let context = surrounding::window(&text, cursor, anchor).unwrap();
        assert!(context.text.len() <= surrounding::MAX_BYTES);
        assert_eq!(context.document_start + context.cursor as usize, cursor);
        assert_eq!(context.document_start + context.anchor as usize, anchor);
        assert!(context.text.is_char_boundary(context.cursor as usize));
        assert!(context.text.is_char_boundary(context.anchor as usize));
        assert_eq!(
            &text[context.document_start..context.document_start + context.text.len()],
            context.text
        );
    }
}

/// 【输入法测试】【上下文窗口】短文档不丢文字，超长选区、非法字节及窗口内空字符不发送伪造上下文，无参数、无返回值
#[test]
fn unsupported_surrounding_context_is_not_truncated_into_a_false_selection() {
    let context = surrounding::window("one\ntwo", 5, 1).unwrap();
    assert_eq!(context.text, "one\ntwo");
    assert_eq!(
        (context.cursor, context.anchor, context.document_start),
        (5, 1, 0)
    );
    assert!(surrounding::window(&"x".repeat(5000), 5000, 0).is_none());
    assert!(surrounding::window("\u{754c}", 1, 0).is_none());
    assert!(surrounding::window("x\0y", 1, 1).is_none());
}

/// 【输入法测试】【上下文取整】扫描多字节文档的全部合法位置，窗口截断始终保留光标和选区边界，无参数、无返回值
#[test]
fn context_cropping_never_splits_utf8_or_excludes_its_cursor() {
    let text = "x\u{e9}\u{754c}\n".repeat(800);
    for cursor in (0..=text.len()).filter(|position| text.is_char_boundary(*position)) {
        let context = surrounding::window(&text, cursor, cursor).unwrap();
        assert!(context.text.len() <= 4000);
        assert_eq!(context.document_start + context.cursor as usize, cursor);
        assert!(context.text.is_char_boundary(context.cursor as usize));
    }
}

/// 【输入法测试】【候选位置】预编辑采用 UTF-8 字节，支持隐藏与反向范围，拒绝负数混合或码点中间位置，无参数、无返回值
#[test]
fn preedit_cursor_keeps_utf8_offsets_and_explicit_hidden_state() {
    let text = "\u{62fc}\u{97f3}".to_owned();
    assert_eq!(
        Preedit::from_protocol(text.clone(), 6, 3).unwrap().cursor,
        Some((6, 3))
    );
    assert_eq!(
        Preedit::from_protocol(text.clone(), -1, -1).unwrap().cursor,
        None
    );
    assert!(Preedit::from_protocol(text.clone(), 1, 3).is_none());
    assert!(Preedit::from_protocol(text.clone(), -1, 3).is_none());
    assert!(Preedit::from_protocol(text, 9, 9).is_none());
    assert_eq!(Preedit::default().cursor, Some((0, 0)));
    assert_eq!(
        Preedit::from_protocol("\u{20000}".into(), 4, 4)
            .unwrap()
            .cursor,
        Some((4, 4))
    );
    assert!(Preedit::from_protocol("\u{20000}".into(), 2, 2).is_none());
}

/// 【输入法测试】【原子交付】确认和候选共存，done 之后全部暂存清空，不把前一批删除带入下一批，无参数、无返回值
#[test]
fn done_delivers_one_complete_batch_and_resets_every_pending_value() {
    let mut pending = Pending::default();
    pending.commit(Some("\u{4f60}".into()));
    pending.delete(2, 3);
    assert!(pending.preedit(Some("hao".into()), 3, 3));
    let update = pending.done(17, 4, 4);
    assert_eq!(
        (update.target_id, update.serial, update.sync_allowed),
        (17, 4, true)
    );
    assert_eq!(update.batch.commit, "\u{4f60}");
    assert_eq!(
        (update.batch.delete_before, update.batch.delete_after),
        (2, 3)
    );
    assert_eq!(update.batch.preedit.text, "hao");
    assert_eq!(pending.done(17, 4, 4).batch, Batch::default());
}

/// 【输入法测试】【旧序号】与当前请求数量不匹配的 done 仍交付确认，只禁止重发状态，不丢输入法文字，无参数、无返回值
#[test]
fn an_old_done_serial_delivers_text_without_allowing_context_resynchronization() {
    let mut pending = Pending::default();
    pending.commit(Some("\u{4f60}".into()));
    let update = pending.done(17, u32::MAX, 0);
    assert_eq!(update.batch.commit, "\u{4f60}");
    assert!(!update.sync_allowed);
}

/// 【输入法测试】【候选错误隔离】非法后续候选不能沿用旧候选，也不能丢掉同一批已经确认的文字，无参数、无返回值
#[test]
fn an_invalid_preedit_does_not_replay_old_candidates_or_discard_committed_text() {
    let mut pending = Pending::default();
    pending.commit(Some("\u{4f60}".into()));
    assert!(pending.preedit(Some("ni".into()), 2, 2));
    assert!(!pending.preedit(Some("\u{754c}".into()), 1, 1));
    let update = pending.done(17, 2, 2);
    assert_eq!(update.batch.commit, "\u{4f60}");
    assert_eq!(update.batch.preedit, Preedit::default());
    pending.commit(Some("old".into()));
    pending.reset();
    assert_eq!(pending.done(18, 3, 3).batch, Batch::default());
}

/// 【输入法测试】【原版事实】从十七组真实 Qt 样本核对候选不写正文、选区删除和独立确认撤销，未接入生产路由时不把它视为产品通过，无参数、无返回值
#[test]
fn frozen_qt_events_define_the_required_composition_and_history_contract() {
    let rows: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/qt-text-input-method.json")).unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 17);
    assert_eq!(
        rows.iter()
            .map(|row| row["states"].as_array().unwrap().len())
            .sum::<usize>(),
        77
    );
    let get = |name| rows.iter().find(|row| row["name"] == name).unwrap();
    let cancel = get("preedit-cancel");
    assert_eq!(cancel["states"][1]["editor_text"], "");
    assert_eq!(cancel["states"][1]["preedit"], "ni");
    assert_eq!(
        cancel["states"][2]["undo_steps"],
        cancel["states"][0]["undo_steps"]
    );
    let selection = get("selection-preedit-cancel");
    assert_eq!(selection["states"][1]["editor_text"], "alpeta");
    assert_eq!(selection["states"][2]["editor_text"], "alpeta");
    assert_eq!(selection["states"][3]["editor_text"], "alpha beta");
    assert_eq!(
        selection["states"][3]["anchor"],
        selection["states"][3]["editor_cursor"]
    );
    let confirmed = get("unicode-direct");
    assert_eq!(confirmed["states"][2]["editor_text"], "\u{4f60}\u{597d}");
    assert_eq!(confirmed["states"][3]["editor_text"], "\u{4f60}");
    assert_eq!(confirmed["states"][4]["editor_text"], "");
    let multi = get("multiline-preedit");
    assert_eq!(multi["states"][1]["editor_cursor"], 5);
    assert_eq!(multi["states"][1]["surrounding_cursor"], 1);
    assert_eq!(
        multi["states"][2]["editor_text"],
        "one\nt\u{4e2d}\u{6587}wo"
    );
    let empty = get("empty-event-with-selection");
    assert_eq!(empty["states"][1]["editor_text"], "alpha beta");
    assert_eq!(empty["states"][1]["anchor"], 3);
    let cross = get("cross-paragraph-selection");
    assert_eq!(cross["states"][0]["surrounding_anchor"], -6);
    assert_eq!(cross["states"][1]["editor_text"], "onhree");
    let supplementary = get("supplementary-preedit");
    assert_eq!(supplementary["states"][2]["editor_text"], "\u{20000}");
    assert_eq!(supplementary["states"][2]["editor_cursor"], 2);
}
