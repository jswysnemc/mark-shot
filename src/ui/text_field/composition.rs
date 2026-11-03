//! 【字段输入法】【原子编辑】先计算完整替换和验证结果，再交付正文与候选；单行撤销不同于正文标注

use super::{
    LineEditState,
    history::{Delta, History, Kind, Position},
    validation::Rule,
};
use crate::types::text_input::{Edit, Preedit, Replacement};

#[derive(Clone, Debug)]
pub struct Composition {
    pub preedit: Preedit,
    pub position: usize,
    pub display: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Update {
    pub accepted: bool,
    pub text_changed: bool,
    pub changed: bool,
}

struct Step {
    delta: Delta,
    before: Position,
    after: Position,
    kind: Kind,
}
struct Plan {
    text: String,
    position: Position,
    steps: Vec<Step>,
    accepted: bool,
    rewritten: bool,
}

/// 【字段输入法】【字符位置】将 text 的 byte 光标转换为字符位置，anchor 保留选区；非法字节范围返回空值
fn position(text: &str, byte: usize, anchor: Option<usize>) -> Option<Position> {
    let cursor = text.get(..byte)?.chars().count();
    let anchor = match anchor {
        Some(anchor) => Some(text.get(..anchor)?.chars().count()),
        None => None,
    };
    Some(Position { cursor, anchor })
}

/// 【字段输入法】【完整计划】按 field、event、rule 返回替换与验证计划，非法编码范围返回空值；计算阶段不修改任何字段状态
fn prepare(field: &LineEditState, event: &Edit, rule: Rule) -> Option<Plan> {
    if let Some((begin, end)) = event.preedit.cursor
        && (!event.preedit.text.is_char_boundary(begin)
            || !event.preedit.text.is_char_boundary(end))
    {
        return None;
    }
    let count = field.text.chars().count();
    if field.cursor > count || field.selection_anchor.is_some_and(|anchor| anchor > count) {
        return None;
    }
    let old_preedit = field
        .composition
        .as_ref()
        .map_or("", |composition| &composition.preedit.text);
    let replacement = event.replacement.unwrap_or(Replacement {
        start: 0,
        length: 0,
    });
    let editing =
        !event.commit.is_empty() || replacement.length != 0 || event.preedit.text != old_preedit;
    let mut text = field.text.clone();
    let mut cursor = field.cursor_byte();
    let mut current = field.position();
    let mut steps = Vec::new();
    // 1. 【字段输入法】【选区删除】非空输入先收起选区，取消旧候选也属于输入；初始空事件保持选区
    if editing && let Some((start, end)) = field.selection_byte_range() {
        let removed = text.get(start..end)?.to_owned();
        text.replace_range(start..end, "");
        let after = position(&text, start, None)?;
        steps.push(Step {
            delta: Delta {
                start,
                removed,
                inserted: String::new(),
            },
            before: current,
            after,
            kind: Kind::SelectionDelete,
        });
        cursor = start;
        current = after;
    }
    // 2. 【字段输入法】【相对替换】替换从选区收起后的位置计算，撤销恢复实际替换选区而非旧正文光标
    if editing && (!event.commit.is_empty() || replacement.start != 0 || replacement.length != 0) {
        let start = cursor.checked_add_signed(replacement.start)?;
        let end = start.checked_add(replacement.length)?;
        let removed = text.get(start..end)?.to_owned();
        let allowance = rule.maximum().saturating_sub(
            text.encode_utf16()
                .count()
                .saturating_sub(removed.encode_utf16().count()),
        );
        let inserted = Rule::clipped(&event.commit, allowance).to_owned();
        let before = position(&text, start, (start != end).then_some(end))?;
        text.replace_range(start..end, &inserted);
        let after = position(&text, start + inserted.len(), None)?;
        let kind = if inserted.is_empty() {
            Kind::SelectionDelete
        } else {
            Kind::Insert
        };
        steps.push(Step {
            delta: Delta {
                start,
                removed,
                inserted,
            },
            before,
            after,
            kind,
        });
        current = after;
    }
    // 3. 【字段输入法】【整批验证】验证器可以补回完整单位；非法确认回退本批正文和历史，但保留新候选与原版回退选区
    let rewritten = rule.restore_suffix(&mut text);
    let accepted = rule.accepts(&text);
    if !accepted {
        current = steps.first().map_or(field.position(), |step| step.before);
    }
    Some(Plan {
        text,
        position: current,
        steps,
        accepted,
        rewritten,
    })
}

/// 【字段输入法】【计划执行】将 event 应用于 field，rule 指定验证器，返回是否接受正文；非法字节事件整体拒绝，合法新候选独立显示
pub(super) fn apply(field: &mut LineEditState, event: Edit, rule: Rule) -> Update {
    let Some(plan) = prepare(field, &event, rule) else {
        return Update::default();
    };
    let text_changed = plan.accepted
        && (plan.rewritten
            || plan
                .steps
                .iter()
                .any(|step| !step.delta.removed.is_empty() || !step.delta.inserted.is_empty()));
    let preedit_changed = field
        .composition
        .as_ref()
        .map_or(!event.preedit.text.is_empty(), |composition| {
            composition.preedit != event.preedit
        });
    let changed = text_changed || preedit_changed || field.position() != plan.position;
    if changed {
        field.revision = field.revision.wrapping_add(1);
    }
    if plan.accepted {
        if plan.rewritten {
            field.history = History::default();
        } else {
            for step in plan.steps {
                field
                    .history
                    .record(step.delta, step.before, step.after, step.kind);
            }
        }
        field.text = plan.text;
    }
    field.cursor = plan.position.cursor;
    field.selection_anchor = plan.position.anchor;
    field.composition = None;
    if !event.preedit.text.is_empty() {
        let at = field.cursor_byte();
        let mut display = field.text.replace(['\n', '\r'], " ");
        display.insert_str(at, &event.preedit.text.replace(['\n', '\r'], " "));
        field.composition = Some(Composition {
            preedit: event.preedit,
            position: at,
            display,
        });
    }
    Update {
        accepted: plan.accepted,
        text_changed,
        changed,
    }
}
