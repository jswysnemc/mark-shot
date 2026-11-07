//! 【文字识别】【表格导出】生成可直接粘贴的电子表格与文档格式

use super::table::Table;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Tsv,
    Csv,
    Markdown,
    Html,
}

impl Format {
    /// 【文字识别】【格式名称】返回当前导出格式的界面名称
    pub fn label(self) -> &'static str {
        match self {
            Self::Tsv => "Copy as spreadsheet (TSV)",
            Self::Csv => "Copy as CSV",
            Self::Markdown => "Copy as Markdown",
            Self::Html => "Copy as HTML",
        }
    }
}

/// 【文字识别】【字段转义】按 separator 转义 value 内的分隔符、引号和换行；返回完整字段
fn field(value: &str, separator: char) -> String {
    if value.contains([separator, '"', '\r', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.into()
    }
}

/// 【文字识别】【分隔格式】将 table 按 separator、ending 连接；保留空单元格与原始 Unicode 内容
fn delimited(table: &Table, separator: char, ending: &str) -> String {
    let separator_text = separator.to_string();
    table
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| field(cell, separator))
                .collect::<Vec<_>>()
                .join(&separator_text)
        })
        .collect::<Vec<_>>()
        .join(ending)
}

/// 【文字识别】【HTML 转义】将 value 中具有标记含义的字符转义；返回纯文本安全内容
fn html_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "<br>")
}

/// 【文字识别】【Markdown 行】将 row 转换为一行 Markdown；转义竖线、反斜杠与 HTML
fn markdown_row(row: &[String]) -> String {
    let cells = row
        .iter()
        .map(|cell| html_text(cell).replace('\\', "\\\\").replace('|', "\\|"))
        .collect::<Vec<_>>()
        .join(" | ");
    format!("| {cells} |")
}

/// 【文字识别】【表格导出】将 table 转换为 format；首行作为 Markdown 与 HTML 表头
pub fn text(table: &Table, format: Format) -> String {
    match format {
        Format::Tsv => delimited(table, '\t', "\n"),
        Format::Csv => delimited(table, ',', "\r\n"),
        Format::Markdown => {
            let Some((first, rest)) = table.rows.split_first() else {
                return String::new();
            };
            let mut lines = vec![
                markdown_row(first),
                markdown_row(&vec!["---".into(); table.columns()]),
            ];
            lines.extend(rest.iter().map(|row| markdown_row(row)));
            lines.join("\n")
        }
        Format::Html => {
            let mut output = String::from("<table>\n");
            for (index, row) in table.rows.iter().enumerate() {
                let tag = if index == 0 { "th" } else { "td" };
                output.push_str("  <tr>");
                for cell in row {
                    output.push_str(&format!("<{tag}>{}</{tag}>", html_text(cell)));
                }
                output.push_str("</tr>\n");
            }
            output.push_str("</table>");
            output
        }
    }
}
