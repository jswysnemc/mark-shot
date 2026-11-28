//! 【RTF测试】【解析】验证格式、分组、代码页、字段、表格、代码块与暗色画布

use super::super::html::{Align, Block, BlockKind, RichText, Span};
use super::super::rich_text;
use super::parse;

/// 【RTF测试】【原始代码页】无参数，验证 GBK、Shift_JIS 和混合原始转义字节按声明代码页还原，无返回值
#[test]
fn raw_codepage_bytes_keep_multibyte_characters() {
    assert_eq!(
        all_text(&parse(b"{\\rtf1\\ansi\\ansicpg932 \x95\\\\}")),
        "表"
    );
    for (input, expected) in [
        (b"{\\rtf1\\ansi\\ansicpg936 \xd6\xd0}".as_slice(), "中"),
        (
            b"{\\rtf1\\ansi\\ansicpg932 \x93\xfa\x96\\'7b}".as_slice(),
            "日本",
        ),
        (b"{\\rtf1\\ansi\\ansicpg936 \\'d6\xd0}".as_slice(), "中"),
    ] {
        assert_eq!(all_text(&parse(input)), expected);
    }
}

/// 【RTF测试】【段落分组】无参数，验证嵌套段落对齐恢复且末段格式在根组结束后仍保留，无返回值
#[test]
fn paragraph_alignment_restores_after_nested_groups() {
    let rich = parse(r"{\rtf1\qc center\par{\qr right\par}after}");
    assert_eq!(
        rich.blocks
            .iter()
            .map(|block| block.align)
            .collect::<Vec<_>>(),
        [Align::Center, Align::Right, Align::Center]
    );
    assert_eq!(parse(r"{\rtf1\qr final}").blocks[0].align, Align::Right);
}

/// 【RTF测试】【行距保留】无参数，验证倍数和固定行距按字号转换，分组结束及 pard 恢复行距，无返回值
#[test]
fn line_spacing_preserves_multiples_fixed_height_and_scope() {
    let minimum = parse(r"{\rtf1\fs48\sl240\slmult0 A}");
    assert!(
        minimum.blocks[0].spans[0].style.line_height.unwrap() >= crate::theme::font::line_height()
    );
    let rich = parse(r"{\rtf1\fs24\sl480\slmult1 A\par{\sl240 B\par}C\par\pard D}");
    assert_eq!(
        rich.blocks
            .iter()
            .map(|block| block.spans[0].style.line_height)
            .collect::<Vec<_>>(),
        [Some(2.0), Some(1.0), Some(2.0), None]
    );
    assert_eq!(
        parse(r"{\rtf1\fs24\sl-480\slmult0 A}").blocks[0].spans[0]
            .style
            .line_height,
        Some(2.0)
    );
}
use crate::theme::Rgba;

/// 【RTF测试】【文本拼接】按 block 返回所有片段拼接后的文本
fn text_of(block: &Block) -> String {
    block.spans.iter().map(|span| span.text.as_str()).collect()
}

/// 【RTF测试】【全文拼接】按 rich 返回所有块或表格单元片段拼接后的文本
fn all_text(rich: &RichText) -> String {
    let mut text = String::new();
    for block in &rich.blocks {
        if let BlockKind::Table { rows, .. } = &block.kind {
            for row in rows {
                for cell in &row.cells {
                    text.extend(cell.spans.iter().map(|span| span.text.as_str()));
                }
            }
        } else {
            text.push_str(&text_of(block));
        }
    }
    text
}

/// 【RTF测试】【首片段】按 rich 返回第一个普通块的首个片段，不存在时 panic 提示
fn first_span(rich: &RichText) -> &Span {
    rich.blocks
        .first()
        .and_then(|block| block.spans.first())
        .expect("expected a first span")
}

/// 【RTF测试】【基础格式】按 RTF 应用粗斜下划删除线、字号、前景与高亮，返回对应样式
#[test]
fn simple_character_formatting_maps_to_span_style() {
    let rich = parse(
        r"{\rtf1\ansi{\colortbl;\red0\green255\blue0;\red255\green0\blue0;}\pard\b\i\ul\strike\fs24\cf2\highlight1 A\par}",
    );
    let span = first_span(&rich);
    assert_eq!(span.text.trim(), "A");
    assert!(span.style.bold);
    assert!(span.style.italic);
    assert!(span.style.underline);
    assert!(span.style.strike);
    assert_eq!(span.style.size, Some(16.0));
    assert_eq!(span.style.color, Some(Rgba(255, 0, 0, 255)));
    assert_eq!(span.style.background, Some(Rgba(0, 255, 0, 255)));
}

/// 【RTF测试】【分组作用域】按分组内加粗、分组外普通，返回加粗仅在分组内生效
#[test]
fn group_restores_character_formatting() {
    let rich = parse(r"{\rtf1{\b bold} plain}");
    let block = rich.blocks.first().expect("expected one block");
    assert_eq!(text_of(block), "bold plain");
    assert!(block.spans[0].style.bold);
    assert!(block.spans.get(1).is_some_and(|span| !span.style.bold));
}

/// 【RTF测试】【Unicode】按 `\u`、GBK 代码页和 UTF-16 代理对返回正确字符
#[test]
fn unicode_and_codepage_escapes_decode() {
    let unicode = parse(r"{\rtf1\ansi\ansicpg1252 \u20013?}");
    assert_eq!(all_text(&unicode), "中");
    let gbk = parse(r"{\rtf1\ansi\ansicpg936 \'d6\'d0}");
    assert_eq!(all_text(&gbk), "中");
    let pair = parse(r"{\rtf1\u-10179?\u-8704?}");
    assert_eq!(all_text(&pair), "😀");
}

/// 【RTF测试】【段落控制】按换行、制表、居中对齐、隐藏文字与 `\bin` 返回预期文本
#[test]
fn paragraphs_line_tab_alignment_hidden_and_bin() {
    let rich = parse(r"{\rtf1\qc A\line B\tab C\par\v hidden\v0 visible\par}");
    let first = rich.blocks.first().expect("expected first paragraph");
    assert_eq!(first.align, Align::Center);
    assert_eq!(text_of(first), "A\nB\tC");
    let text = all_text(&rich);
    assert!(text.contains("visible"));
    assert!(!text.contains("hidden"));
    let binary = parse(r"{\rtf1 A\bin3XYZB}");
    assert_eq!(all_text(&binary), "AB");
}

/// 【RTF测试】【字段】按 fldinst 丢弃指令、保留 fldrslt，并让 HYPERLINK 结果带链接样式
#[test]
fn fields_keep_result_and_detect_hyperlink() {
    let link =
        parse(r#"{\rtf1 {\field{\*\fldinst HYPERLINK "https://example.com"}{\fldrslt Example}}}"#);
    let span = first_span(&link);
    assert_eq!(span.text, "Example");
    assert!(span.style.link);
    let text = all_text(&link);
    assert!(!text.contains("HYPERLINK"));
    assert!(!text.contains("example.com"));
    let plain = parse(r"{\rtf1 {\field{\*\fldinst PAGE}{\fldrslt 1}}}");
    assert_eq!(all_text(&plain), "1");
    assert!(!first_span(&plain).style.link);
}

/// 【RTF测试】【字体与代码块】按 fonttbl 识别 Consolas，并将连续等宽段落合并为代码块
#[test]
fn font_table_marks_monospace_and_merges_code() {
    let rich = parse(r"{\rtf1{\fonttbl{\f0 Calibri;}{\f1 Consolas;}}\f1 one\par\f1 two\par}");
    let block = rich.blocks.first().expect("expected one code block");
    assert_eq!(block.kind, BlockKind::Code);
    assert_eq!(rich.blocks.len(), 1);
    assert_eq!(text_of(block), "one\ntwo");
    assert!(block.spans.iter().all(|span| span.style.code));
}

/// 【RTF测试】【表格】按 trowd/trhdr/cellx/intbl/cell/row 生成表头行与带背景的两个单元格
#[test]
fn table_rows_and_cell_backgrounds_are_built() {
    let rich = parse(
        r"{\rtf1{\colortbl;\red1\green2\blue3;\red4\green5\blue6;\red7\green8\blue9;}\trowd\trhdr\clcbpat2\cellx3000\clcbpat3\cellx7000\intbl A\cell B\cell\row\pard}",
    );
    let block = rich.blocks.first().expect("expected a table block");
    let BlockKind::Table { rows, style } = &block.kind else {
        panic!("expected table block");
    };
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert!(row.header);
    assert_eq!(row.cells.len(), 2);
    assert_eq!(style.width, None);
    assert!(style.columns.is_empty());
    assert!(!style.fill);
    assert_eq!(style.border, None);
    assert_eq!(
        text_of(&Block {
            kind: BlockKind::Paragraph,
            align: Align::Left,
            background: None,
            spans: row.cells[0].spans.clone(),
        }),
        "A"
    );
    assert_eq!(
        text_of(&Block {
            kind: BlockKind::Paragraph,
            align: Align::Left,
            background: None,
            spans: row.cells[1].spans.clone(),
        }),
        "B"
    );
    assert_eq!(row.cells[0].background, Some(Rgba(4, 5, 6, 255)));
    assert_eq!(row.cells[1].background, Some(Rgba(7, 8, 9, 255)));
    assert!(row.cells.iter().all(|cell| cell.header));
    let image = tiny_skia::Pixmap::decode_png(&rich_text::image(&rich, 1.0).unwrap()).unwrap();
    assert!(image.width() > 0 && image.height() > 0);
}

/// 【RTF测试】【项目符号】按 `\bullet` 生成列表项并补写“• ”标记片段
#[test]
fn bullet_creates_list_item_marker() {
    let rich = parse(r"{\rtf1\pard\bullet item\par}");
    let block = rich.blocks.first().expect("expected a list item");
    assert!(matches!(
        block.kind,
        BlockKind::ListItem {
            depth: 0,
            ref marker
        } if marker == "• "
    ));
    assert_eq!(block.spans[0].text, "• ");
    assert_eq!(text_of(block), "• item");
}

/// 【RTF测试】【暗色画布】按深色段落背景与浅色文字返回暗画布，且渲染出暗色与浅色像素
#[test]
fn dark_paragraph_background_sets_canvas_and_renders() {
    let rich = parse(
        r"{\rtf1{\colortbl;\red10\green20\blue30;\red240\green240\blue240;}\pard\cb1\cf2 dark text\par}",
    );
    assert_eq!(rich.background, Some(Rgba(10, 20, 30, 255)));
    assert!(
        rich.blocks
            .iter()
            .flat_map(|block| &block.spans)
            .all(|span| span.style.background.is_none())
    );
    let png = rich_text::image(&rich, 1.0).expect("expected a rendered png");
    let image = tiny_skia::Pixmap::decode_png(&png).expect("expected a decodable png");
    let dark = image
        .pixels()
        .iter()
        .filter(|pixel| {
            pixel.alpha() > 200 && pixel.red() < 60 && pixel.green() < 70 && pixel.blue() < 90
        })
        .count();
    let light = image
        .pixels()
        .iter()
        .filter(|pixel| pixel.alpha() > 200 && pixel.red() > 200 && pixel.green() > 200)
        .count();
    assert!(
        dark > 1000,
        "expected a dark canvas, got {dark} dark pixels"
    );
    assert!(light > 20, "expected light text, got {light} light pixels");
}

/// 【RTF测试】【空与畸形】按空输入返回空内容，按任意畸形输入解析不 panic
#[test]
fn empty_and_malformed_inputs_do_not_panic() {
    assert!(parse("").blocks.is_empty());
    let garbage = parse(r"{\rtf1 {{{ \bogus \\ \u999999 \'zz \bin-5 }}} trailing");
    let _ = all_text(&garbage);
    let unclosed = parse(r"{\rtf1\b bold\par {\i nested");
    assert!(all_text(&unclosed).contains("bold"));
}

/// 【RTF测试】【逐格段落】按 Word 常见的 `\pard\intbl` 单元格重置，返回完整多行表格
#[test]
fn word_style_cell_paragraph_resets_keep_the_table() {
    let rich = parse(
        r"{\rtf1\ansi{\fonttbl{\f0 Calibri;}}{\colortbl;\red238\green240\blue243;}
\trowd\trhdr\cellx3000\cellx7000
\pard\intbl\b Document\cell\pard\intbl Content\cell\row
\trowd\clcbpat1\cellx3000\cellx7000
\pard\intbl Installation\cell\pard\intbl Build dependencies\cell\row
\pard after\par}",
    );
    let table = rich
        .blocks
        .iter()
        .find_map(|block| match &block.kind {
            BlockKind::Table { rows, .. } => Some(rows),
            _ => None,
        })
        .expect("expected one table block");
    assert_eq!(table.len(), 2);
    assert_eq!(table[0].cells.len(), 2);
    assert_eq!(table[1].cells.len(), 2);
    assert!(table[0].header);
    assert_eq!(table[1].cells[0].background, Some(Rgba(238, 240, 243, 255)));
    assert_eq!(table[1].cells[0].spans[0].text.trim(), "Installation");
    assert_eq!(table[1].cells[1].spans[0].text.trim(), "Build dependencies");
    assert!(all_text(&rich).contains("after"));
}
