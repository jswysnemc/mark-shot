//! 【选区尺寸】【词语导航】数字段连续移动，正号和分组符单独选择，删除仍通过字段验证

use super::field::NumberField;

/// 【选区尺寸】【词语范围】按 text 与 character 返回点击字符所在数字段或单个分隔符，空文本返回零范围
pub fn word(text: &str, character: usize) -> (usize, usize) {
    let chars: Vec<_> = text.chars().collect();
    if chars.is_empty() {
        return (0, 0);
    }
    let character = character.min(chars.len() - 1);
    if !chars[character].is_ascii_digit() {
        return (character, character + 1);
    }
    let mut start = character;
    let mut end = character + 1;
    while start > 0 && chars[start - 1].is_ascii_digit() {
        start -= 1;
    }
    while end < chars.len() && chars[end].is_ascii_digit() {
        end += 1;
    }
    (start, end)
}

/// 【选区尺寸】【词语边界】按 text、cursor 与 backward 返回前一或后一数字段边界，分隔符只跨越一个字符
pub fn boundary(text: &str, cursor: usize, backward: bool) -> usize {
    let length = text.chars().count();
    let cursor = cursor.min(length);
    if backward {
        if cursor == 0 {
            return 0;
        }
        word(text, cursor - 1).0
    } else if cursor == length {
        length
    } else {
        word(text, cursor).1
    }
}

/// 【选区尺寸】【词语移动】按 backward、extend 移动 field 活动光标，选区方向沿用原锚点，无返回值
pub fn move_word(field: &mut NumberField, backward: bool, extend: bool) {
    let next = boundary(&field.editor.text, field.editor.cursor, backward);
    field.edit(|editor| editor.move_to(next, extend));
}

/// 【选区尺寸】【词语删除】按 backward 删除 field 已选文字或相邻数字段，验证失败保留新词语选择，无返回值
pub fn delete_word(field: &mut NumberField, backward: bool) {
    // 1. 【选区尺寸】【删除范围】已有选择直接使用；没有选择时先建立词语选区，不能把导航与删除一起回退
    if field.editor.selection_range().is_none() {
        move_word(field, backward, true);
    }
    // 2. 【选区尺寸】【原子删除】非法剩余正文保持不变，已经建立的选择范围与原版一致
    field.edit(|editor| editor.backspace_selection_only());
}

/// 【选区尺寸】【选择后退格】在 field 中按 Ctrl+Shift+Backspace 扩展前一词语再删除，零长度选择按向前删除处理，无返回值
pub fn extend_delete_backward(field: &mut NumberField) {
    move_word(field, true, true);
    field.edit(|editor| editor.delete_forward());
}
