//! 【标注】【文字选择】提取当前文字选区

use super::*;

pub(super) fn extract_selected_text(editor: &Editor<'static>) -> Option<String> {
    let (start, end) = editor.selection_bounds()?;

    let result = editor.with_buffer(|buffer| {
        let mut result = String::new();

        for (i, line) in buffer.lines.iter().enumerate() {
            if i >= start.line && i <= end.line {
                let text = line.text();

                let start_idx = if i == start.line { start.index } else { 0 };
                let end_idx = if i == end.line { end.index } else { text.len() };

                let safe_start = start_idx.min(text.len());
                let safe_end = end_idx.min(text.len());

                result.push_str(&text[safe_start..safe_end]);

                if i < end.line {
                    result.push('\n');
                }
            }
        }

        result
    });

    if result.is_empty() {
        None
    } else {
        Some(crate::editor::text_session::plain_text(&result))
    }
}
