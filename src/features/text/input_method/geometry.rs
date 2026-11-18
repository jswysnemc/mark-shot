//! 【输入法】【字符布局】按真实显示编辑器读取组合字符边界，不使用平均字宽估算位置

use crate::types::text_input::CompositionGeometry;
use cosmic_text::{Cursor, Edit as _, Editor};

/// 【输入法】【组合位置】按 editor、start 和 text 返回 UTF-16 字符边界几何，project 将字形坐标与行高转换为目标表面矩形
pub fn layout(
    editor: &Editor<'_>,
    start: Cursor,
    text: &str,
    mut project: impl FnMut(f32, f32, f32) -> (i32, i32, i32, i32),
) -> CompositionGeometry {
    let positions = editor.with_buffer(|buffer| {
        let mut cursor = start;
        let mut offset = 0;
        let mut positions = Vec::new();
        let mut carriage_return = false;
        // 1. 【输入法】【字符边界】包含首字和末尾光标位置，代理对内部不会生成伪造坐标
        for character in text.chars().map(Some).chain(std::iter::once(None)) {
            if let Some(rect) = buffer.layout_runs().find_map(|run| {
                run.cursor_position(&cursor)
                    .map(|x| project(x, run.line_top, run.line_height))
            }) {
                positions.push((offset, rect));
            }
            if let Some(character) = character {
                offset += character.len_utf16() as u32;
                cursor = if character == '\r' || character == '\n' && !carriage_return {
                    Cursor::new(cursor.line + 1, 0)
                } else if character == '\n' && carriage_return {
                    cursor
                } else {
                    Cursor::new(cursor.line, cursor.index + character.len_utf8())
                };
                carriage_return = character == '\r';
            }
        }
        positions
    });
    CompositionGeometry { positions }
}
