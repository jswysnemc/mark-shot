//! 【历史选区测试】【输入回归】前后切换、跨会话存储和输入字段互不干扰

use super::*;
use tiny_skia::Rect;

/// 【历史选区测试】【两个选区】无参数，返回记录两个不同位置的编辑器和对应矩形
fn fixture() -> (EditorState, Rect, Rect) {
    let mut state = editor();
    let a = Rect::from_xywh(30.0, 40.0, 120.0, 80.0).unwrap();
    let b = Rect::from_xywh(200.0, 140.0, 200.0, 100.0).unwrap();
    for rect in [a, b] {
        state.selection.zone = Some(rect);
        crate::app::selection_history::record(&mut state);
    }
    (state, a, b)
}

/// 【历史选区测试】【快捷前后】逗号恢复上一条，句号恢复下一条，切换不触发截图完成或更改标注历史
#[test]
fn punctuation_browses_without_capturing_or_recording_duplicates() {
    let (mut state, a, b) = fixture();
    key(&mut state, ",", &mut 0);
    assert_eq!(state.selection.zone, Some(a));
    key(&mut state, ",", &mut 0);
    assert_eq!(state.selection.zone, Some(a));
    key(&mut state, ".", &mut 0);
    assert_eq!(state.selection.zone, Some(b));
    key(&mut state, ".", &mut 0);
    assert_eq!(state.selection.zone, Some(b));
    assert!(state.finish.is_none() && state.undo_stack.is_empty());
}

/// 【历史选区测试】【跨会话】新会话首次按逗号取最近选区，改变显示器布局后不恢复旧坐标
#[test]
fn persisted_history_reopens_latest_only_on_matching_displays() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("history.json");
    let (mut state, a, b) = fixture();
    state.selection_history = crate::features::selection_history::History::load(path.clone());
    for rect in [a, b, b] {
        state.selection.zone = Some(rect);
        crate::app::selection_history::record(&mut state);
    }
    state.selection_history = crate::features::selection_history::History::load(path);
    state.selection.zone = None;
    key(&mut state, ",", &mut 0);
    assert_eq!(state.selection.zone, Some(b));
    key(&mut state, ",", &mut 0);
    assert_eq!(state.selection.zone, Some(a));
    state.selection.zone = None;
    state.placements[0].position = (640, 0);
    key(&mut state, ",", &mut 0);
    assert!(state.selection.zone.is_none());
}

/// 【历史选区测试】【文字优先】正在输入正文时逗号和句号写入文字，不切换历史选区
#[test]
fn text_editor_owns_punctuation() {
    let (mut state, _, b) = fixture();
    crate::editor::text_session::begin_new(&mut state, (240.0, 160.0), 1.0, &mut 0);
    for text in [",", "."] {
        super::super::handle_key(
            &mut state,
            Some(crate::keys::parse(text).unwrap()),
            Some(text.into()),
            &mut 0,
        );
    }
    assert_eq!(state.selection.zone, Some(b));
    assert!(state.text_session.is_some());
}

/// 【历史选区测试】【并发保存】两个旧快照交错保存时合并已有文件，浏览时也能看到另一会话追加的选区
#[test]
fn stale_sessions_merge_interleaved_selections() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("history.json");
    let (state, a, b) = fixture();
    let mut first = crate::features::selection_history::History::load(path.clone());
    let mut second = crate::features::selection_history::History::load(path.clone());
    first.record(a, &state.placements);
    second.record(b, &state.placements);
    let mut reader = crate::features::selection_history::History::load(path);
    assert_eq!(reader.step(false, &state.placements), Some(b));
    assert_eq!(reader.step(false, &state.placements), Some(a));
    assert_eq!(first.step(true, &state.placements), Some(b));
}
