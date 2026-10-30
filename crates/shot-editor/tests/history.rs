use serde_json::json;
use shot_editor::{History, Snapshot};

/// 构造 id 对应的完整标注快照，返回包含全部字段和多选状态的数据。
fn snapshot(id: i32) -> Snapshot {
    serde_json::from_value(json!({
        "version": 1, "nextNumber": id + 7, "nextAnnotationId": id + 2,
        "selectedAnnotationId": id, "selectedAnnotationIds": [id, id + 1],
        "annotations": [{"id": id, "tool": 8, "rect": [-1.5, 2.25, 30.5, 40.0],
            "points": [[0.25, 2.5], [12.75, -1.5]], "text": "示例\ntext", "number": 3,
            "color": 0x80ff4d4d_u32, "backgroundColor": 0x40010203_u32, "width": -2.25,
            "filled": true, "cornerRadius": 7.5, "arrowStyle": 3, "arrowLineStyle": 2,
            "highlighterStyle": 1, "rotationDegrees": 42.25, "magnifierScale": 3.5,
            "magnifierShape": 1, "numberStyle": 6, "fontFamily": "Noto Sans CJK SC",
            "fontWeight": 700, "textItalic": true, "rectangleStyle": 3, "markerShape": 23,
            "mosaicStyle": 4, "rulerMode": 1}]
    }))
    .unwrap()
}

#[test]
/// 验证完整标注快照在编码及撤销重做中保持全部字段，无参数、无返回值。
fn all_annotation_fields_survive_undo_redo() {
    let before = snapshot(1);
    let after = snapshot(2);
    let encoded = serde_json::to_vec(&before).unwrap();
    assert_eq!(Snapshot::decode(&encoded).unwrap(), before);
    let mut history = History::new(100).unwrap();
    history.checkpoint(before.clone()).unwrap();
    assert_eq!(
        history.step(after.clone(), false).unwrap(),
        Some(before.clone())
    );
    assert_eq!(history.step(before, true).unwrap(), Some(after));
}

#[test]
/// 验证非整像素坐标经 JSON 往返后保留完整双精度，无参数、无返回值。
fn fractional_coordinates_round_trip_without_precision_drift() {
    let mut before = snapshot(1);
    let mut seed = 0x1234abcd9876_u64;
    for _ in 0..2048 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let value =
            (f64::from_bits(0x3ff0000000000000 | (seed & 0x000fffffffffffff)) - 1.0) * 4096.0;
        before.annotations[0].rect[0] = value;
        before.annotations[0].points[0][1] = -value / 3.0;
        let encoded = serde_json::to_vec(&before).unwrap();
        let restored = Snapshot::decode(&encoded).unwrap();
        assert_eq!(restored.annotations[0].rect[0].to_bits(), value.to_bits());
        assert_eq!(
            restored.annotations[0].points[0][1].to_bits(),
            (-value / 3.0).to_bits()
        );
    }
}

#[test]
/// 验证同一滚轮上下文只记录首次快照，切换上下文先提交旧事务，无参数、无返回值。
fn wheel_transactions_coalesce_and_switch_contexts() {
    let mut history = History::new(100).unwrap();
    assert!(history.needs_snapshot(11));
    history.queue(11, snapshot(1)).unwrap();
    assert!(!history.needs_snapshot(11));
    assert!(history.needs_snapshot(22));
    history.queue(11, snapshot(2)).unwrap();
    assert_eq!(history.info(), (0, 0, Some(11)));
    assert_eq!(history.candidate(false), Some(&snapshot(1)));
    history.queue(22, snapshot(3)).unwrap();
    assert_eq!(history.info(), (1, 0, Some(22)));
    assert_eq!(history.step(snapshot(4), false).unwrap(), Some(snapshot(3)));
    assert_eq!(history.step(snapshot(3), false).unwrap(), Some(snapshot(1)));
    assert_eq!(history.step(snapshot(1), true).unwrap(), Some(snapshot(3)));
}

#[test]
/// 验证新编辑截断重做分支，清空时同时移除待提交事务，无参数、无返回值。
fn branching_and_clear_do_not_resurrect_old_documents() {
    let mut history = History::new(100).unwrap();
    history.checkpoint(snapshot(1)).unwrap();
    history.step(snapshot(2), false).unwrap();
    history.queue(1, snapshot(1)).unwrap();
    assert!(history.candidate(true).is_none());
    assert!(history.step(snapshot(3), true).unwrap().is_none());
    assert_eq!(history.info(), (1, 0, None));
    history.queue(2, snapshot(3)).unwrap();
    history.clear();
    assert!(!history.commit());
    assert_eq!(history.info(), (0, 0, None));
    assert!(history.step(snapshot(4), false).unwrap().is_none());
}

#[test]
/// 验证非法快照不会提交挂起事务或消耗撤销记录，无参数、无返回值。
fn invalid_input_preserves_stacks_and_pending_transaction() {
    let mut history = History::new(100).unwrap();
    history.queue(8, snapshot(1)).unwrap();
    let mut invalid = snapshot(2);
    invalid.annotations[0].points[0][0] = f64::NAN;
    assert!(history.step(invalid.clone(), false).is_err());
    assert!(history.checkpoint(invalid.clone()).is_err());
    assert!(history.queue(9, invalid).is_err());
    assert_eq!(history.info(), (0, 0, Some(8)));
    assert_eq!(history.candidate(false), Some(&snapshot(1)));
    assert!(Snapshot::decode(br#"{"version":999}"#).is_err());
}

#[test]
/// 验证容量限制只丢弃最早快照，独立历史会话互不影响，无参数、无返回值。
fn history_capacity_and_session_ownership_are_bounded() {
    assert!(History::new(0).is_err());
    assert!(History::new(4097).is_err());
    let mut history = History::new(2).unwrap();
    for id in 1..=3 {
        history.checkpoint(snapshot(id)).unwrap();
    }
    let other = History::new(2).unwrap();
    assert_eq!(other.info(), (0, 0, None));
    assert_eq!(history.step(snapshot(4), false).unwrap(), Some(snapshot(3)));
    assert_eq!(history.step(snapshot(3), false).unwrap(), Some(snapshot(2)));
    assert!(history.step(snapshot(2), false).unwrap().is_none());
}

#[test]
/// 验证历史按真实分配容量限制内存，而非仅统计向量有效长度，无参数、无返回值。
fn memory_budget_accounts_for_reserved_capacity() {
    let mut history = History::new(100).unwrap();
    for _ in 0..6 {
        let snapshot = Snapshot {
            selected_annotation_ids: Vec::with_capacity(6 * 1024 * 1024),
            ..Snapshot::default()
        };
        history.checkpoint(snapshot).unwrap();
    }
    assert_eq!(history.info().0, 5);
    let oversized = Snapshot {
        selected_annotation_ids: Vec::with_capacity(9 * 1024 * 1024),
        ..Snapshot::default()
    };
    assert!(history.checkpoint(oversized).is_err());
    assert_eq!(history.info().0, 5);
}

#[test]
/// 验证内存紧张时优先移除较远记录，不能丢弃刚重做操作的撤销目标，无参数、无返回值。
fn memory_pressure_preserves_the_immediate_undo_target() {
    let reserved = |number: i32, mib: usize| Snapshot {
        next_number: number,
        selected_annotation_ids: Vec::with_capacity(mib * 1024 * 1024 / 4),
        ..Snapshot::default()
    };
    let mut history = History::new(100).unwrap();
    for number in 1..=5 {
        history.checkpoint(reserved(number, 25)).unwrap();
    }
    for number in 10..15 {
        history.step(reserved(number, 25), false).unwrap();
    }
    assert_eq!(history.info(), (0, 5, None));
    history.step(reserved(99, 31), true).unwrap();
    assert_eq!(history.info().0, 1);
    assert_eq!(
        history
            .step(Snapshot::default(), false)
            .unwrap()
            .unwrap()
            .next_number,
        99
    );
}
