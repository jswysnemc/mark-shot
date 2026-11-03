use mark_shot::features::ocr_result::{
    export::{self, Format},
    table::{self, Table},
};
use mark_shot::ocr::OcrLine;
use tiny_skia::Rect;

/// 【表格测试】【文字框】构造 text 与指定 x、y、width 对应的识别框；返回固定高度文字行
fn line(text: &str, x: f32, y: f32, width: f32) -> OcrLine {
    OcrLine {
        text: text.into(),
        bounds: Rect::from_xywh(x, y, width, 18.0).unwrap(),
        char_x: vec![],
    }
}

/// 【表格测试】【空列保留】验证输入顺序与缺失单元格不影响列位置
#[test]
fn infers_rows_and_preserves_missing_middle_cells() {
    let mut lines = vec![
        line("名称", 20.0, 10.0, 60.0),
        line("数量", 140.0, 10.5, 40.0),
        line("金额", 260.0, 10.0, 40.0),
        line("纸张", 20.0, 42.0, 60.0),
        line("12", 150.0, 42.0, 30.0),
        line("36", 270.0, 42.0, 30.0),
        line("空列", 20.0, 74.0, 60.0),
        line("15", 270.0, 74.0, 30.0),
    ];
    let expected = table::infer(&lines).unwrap();
    assert_eq!(expected.rows[2], ["空列", "", "15"]);
    lines.reverse();
    assert_eq!(table::infer(&lines), Some(expected));
}

/// 【表格测试】【段落排除】验证单列段落与单行文字不会出现表格页
#[test]
fn ignores_prose_and_single_row() {
    assert!(
        table::infer(&[
            line("first", 0.0, 0.0, 200.0),
            line("second", 0.0, 30.0, 180.0)
        ])
        .is_none()
    );
    assert!(table::infer(&[line("a", 0.0, 0.0, 20.0), line("b", 100.0, 0.0, 20.0)]).is_none());
}

/// 【表格测试】【分隔字段】验证 CSV 与 TSV 中的引号、换行和空值
#[test]
fn csv_and_tsv_quote_fields_without_losing_text() {
    let table = Table {
        rows: vec![vec!["a,b".into(), "引号\"\n换行".into(), "".into()]],
    };
    assert_eq!(
        export::text(&table, Format::Csv),
        "\"a,b\",\"引号\"\"\n换行\","
    );
    let table = Table {
        rows: vec![vec!["a\tb".into(), "".into()]],
    };
    assert_eq!(export::text(&table, Format::Tsv), "\"a\tb\"\t");
}

/// 【表格测试】【标记转义】验证用户文字不会变成 HTML 标记或 Markdown 列分隔符
#[test]
fn escapes_html_and_markdown_content() {
    let table = Table {
        rows: vec![
            vec!["<script>&".into(), "a|b\\c".into()],
            vec!["x\ny".into(), "".into()],
        ],
    };
    let html = export::text(&table, Format::Html);
    assert!(html.contains("&lt;script&gt;&amp;"));
    assert!(!html.contains("<script>"));
    let markdown = export::text(&table, Format::Markdown);
    assert!(markdown.contains("a\\|b\\\\c"));
    assert!(markdown.contains("| --- | --- |"));
    assert!(markdown.contains("x<br>y"));
}

/// 【表格测试】【边界修改】验证单元格更新保留矩形结构，越界不修改内容
#[test]
fn editing_checks_bounds() {
    let mut table = Table {
        rows: vec![vec!["old".into()]],
    };
    assert!(table.set_cell(0, 0, "new".into()));
    assert!(!table.set_cell(1, 0, "bad".into()));
    assert_eq!(table.rows, vec![vec!["new"]]);
}
