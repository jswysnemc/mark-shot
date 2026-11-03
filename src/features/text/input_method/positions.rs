//! 【输入法】【正文位置】在 UTF-8 文档字节与编辑器行位置之间换算，不按字符数假定字节长度

use cosmic_text::Cursor;

/// 【输入法】【文档偏移】按 text 与 cursor 返回完整文档字节位置；不存在的行或字符中间位置返回空值
pub fn document_offset(text: &str, cursor: Cursor) -> Option<usize> {
    let mut offset = 0;
    for (line, value) in text.split('\n').enumerate() {
        if line == cursor.line {
            return value
                .is_char_boundary(cursor.index)
                .then(|| offset + cursor.index);
        }
        offset += value.len() + 1;
    }
    None
}

/// 【输入法】【编辑位置】按 text 与 offset 文档字节返回行光标；换行字节归属上一行尾，非法 UTF-8 边界返回空值
pub fn editor_cursor(text: &str, offset: usize) -> Option<Cursor> {
    if !text.is_char_boundary(offset) {
        return None;
    }
    let mut start = 0;
    for (line, value) in text.split('\n').enumerate() {
        if offset <= start + value.len() {
            return Some(Cursor::new(line, offset - start));
        }
        start += value.len() + 1;
    }
    None
}

/// 【输入法】【前后删除】按 text、cursor 与 before、after 字节长度返回连续删除边界；超出正文或切开码点时返回空值，不部分删除
pub fn deletion_range(
    text: &str,
    cursor: usize,
    before: u32,
    after: u32,
) -> Option<(Cursor, Cursor)> {
    if !text.is_char_boundary(cursor) {
        return None;
    }
    let start = cursor.checked_sub(usize::try_from(before).ok()?)?;
    let end = cursor.checked_add(usize::try_from(after).ok()?)?;
    Some((editor_cursor(text, start)?, editor_cursor(text, end)?))
}
