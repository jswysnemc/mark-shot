//! 【选区尺寸测试】【字段状态】按生产控件操作顺序核对待提交文字、实际尺寸和撤销状态

use super::*;
use mark_shot::ui::selection_size::{field::NumberField, state::Panel};

/// 【选区尺寸测试】【字段快照】按 field 返回生产参考使用的字段状态，不把候选文字当作已提交值
fn snapshot(field: &NumberField) -> Value {
    serde_json::json!({
        "value": field.value,
        "editing": field.editing,
        "text": field.editor.text,
        "cursor": field.editor.cursor,
        "selected": field.editor.selected_text().unwrap_or_default(),
        "selection_start": field.editor.selection_range().map_or(-1, |(start, _)| start as i64),
        "acceptable": field.rule().acceptable(&field.editor.text),
        "undo": field.editor.can_undo(),
        "redo": field.editor.can_redo(),
    })
}

/// 【选区尺寸测试】【状态核对】按 name 对照 panel 与原版同名快照，popup 表示原版比例弹层是否显示，无返回值
fn assert_state(panel: &Panel, name: &str, popup: bool) {
    let fixture = reference();
    let mut expected = fixture["states"]
        .as_array()
        .unwrap()
        .iter()
        .find(|state| state["name"] == name)
        .unwrap()
        .clone();
    assert_eq!(panel.ratio, expected["ratio"].as_f64().unwrap(), "{name}");
    expected.as_object_mut().unwrap().remove("ratio");
    let actual = serde_json::json!({
        "name": name,
        "size": [panel.values.0, panel.values.1],
        "popup": popup,
        "width": snapshot(&panel.fields[0]),
        "height": snapshot(&panel.fields[1]),
    });
    assert_eq!(actual, expected, "{name}");
}

/// 【选区尺寸测试】【控件应用】按 side 和 value 复现原版探针的尺寸回写，无返回值，不调用完整截图窗口控制器
fn apply(panel: &mut Panel, side: ratio::Field, value: Option<i32>) {
    if let Some(value) = value {
        panel.sync(
            panel.requested(side, value),
            panel.maximum,
            panel.ratio,
            panel.language,
        );
    }
}

/// 【选区尺寸测试】【原版操作序列】逐个核对二十个生产控件状态，控件比例回调只同步比例，应用层另测实际缩放
#[test]
fn field_sequence_matches_twenty_real_qt_widget_snapshots() {
    let mut panel = Panel::new(Language::English);
    panel.sync((640, 480), (1200, 800), 0.0, Language::English);
    assert_state(&panel, "initial", false);
    assert_state(&panel, "right_no_edit", false);
    panel.fields[0].begin();
    assert_state(&panel, "width_selected", false);
    assert!(panel.fields[0].edit(|editor| editor.backspace()));
    assert!(panel.fields[0].insert("1201"));
    assert_state(&panel, "over_max_intermediate", false);
    let value = panel.fields[0].commit();
    apply(&mut panel, ratio::Field::Width, value);
    assert_state(&panel, "over_max_committed", false);
    panel.fields[0].begin();
    panel.fields[0].edit(|editor| editor.backspace());
    panel.fields[0].arrow(10);
    assert_state(&panel, "arrow_pending", false);
    panel.fields[0].cancel();
    assert_state(&panel, "escape_discarded", false);
    panel.fields[0].begin();
    panel.fields[0].edit(|editor| editor.backspace());
    assert_state(&panel, "empty_pending", false);
    let value = panel.fields[0].wheel(-1);
    apply(&mut panel, ratio::Field::Width, value);
    assert_state(&panel, "wheel_empty_falls_back", false);
    let value = panel.fields[0].commit();
    apply(&mut panel, ratio::Field::Width, value);
    panel.fields[1].begin();
    assert_state(&panel, "tab_height", false);
    panel.fields[1].edit(|editor| editor.backspace());
    panel.fields[1].insert("300");
    let value = panel.fields[1].commit();
    apply(&mut panel, ratio::Field::Height, value);
    panel.fields[0].begin();
    assert_state(&panel, "backtab_width", false);
    panel.fields[0].cancel();
    let value = panel.fields[1].wheel(-10);
    apply(&mut panel, ratio::Field::Height, value);
    assert_state(&panel, "horizontal_wheel_shift", false);
    assert_state(&panel, "right_lock_ignored", false);
    panel.sync(
        panel.values,
        panel.maximum,
        f64::from(panel.values.0) / f64::from(panel.values.1),
        panel.language,
    );
    assert_state(&panel, "locked_current", false);
    for field in &mut panel.fields {
        field.cancel();
    }
    assert_state(&panel, "ratio_popup", true);
    assert_state(&panel, "right_ratio_ignored", true);
    panel.sync(panel.values, panel.maximum, 21.0 / 9.0, panel.language);
    assert_state(&panel, "ratio_chosen", false);
    panel.fields[0].begin();
    panel.fields[0].edit(|editor| editor.backspace());
    panel.fields[0].insert("1,200");
    assert_state(&panel, "grouped_pending", false);
    let value = panel.fields[0].commit();
    assert_eq!(value, None);
    assert_state(&panel, "grouped_not_committed", false);
    panel.fields[0].begin();
    panel.fields[0].edit(|editor| editor.backspace());
    panel.fields[0].insert("+88");
    let value = panel.fields[0].commit();
    apply(&mut panel, ratio::Field::Width, value);
    assert_state(&panel, "focus_out_committed", false);
}

/// 【选区尺寸测试】【同步边界】用户待提交值保留，滚轮联动接收实际边界回写，程序重置更新输入身份
#[test]
fn user_draft_and_programmatic_wheel_have_distinct_sync_rules() {
    let mut field = NumberField::new(640, 1200);
    field.begin();
    field.insert("900");
    field.sync(600, 1200);
    assert_eq!(field.editor.text, "900");
    let old_identity = field.id;
    assert_eq!(field.wheel(1), Some(901));
    assert_ne!(field.id, old_identity);
    field.sync(700, 1200);
    assert_eq!(field.editor.selected_text().as_deref(), Some("700"));
    field.arrow(10);
    field.sync(650, 1200);
    assert_eq!(field.editor.text, "710");
    assert_eq!(field.value, 650);
    field.cancel();
    assert_eq!(field.value, 650);
}

/// 【选区尺寸测试】【原子编辑】无效粘贴不能部分插入，删除非法符号后的正文也不能越过验证规则
#[test]
fn rejected_edits_preserve_text_selection_and_history() {
    let mut field = NumberField::new(640, 1200);
    field.begin();
    assert!(!field.insert("12x"));
    assert_eq!(field.editor.selected_text().as_deref(), Some("640"));
    assert!(!field.editor.can_undo());
    assert!(field.insert("1,200"));
    assert!(field.edit(|editor| {
        editor.undo();
    }));
    assert_eq!(field.editor.selected_text().as_deref(), Some("640"));
    assert!(field.edit(|editor| {
        editor.redo();
    }));
    assert_eq!(field.editor.text, "1,200");
    assert_eq!(field.commit(), None);
    assert_eq!(field.value, 640);
}
