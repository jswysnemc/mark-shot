use super::*;

/// 【OCR 表头测试】【矩形对照】按 actual、expected 逐项比较实际 Qt 控件整数矩形，无返回值
fn compare(actual: Rect, expected: &serde_json::Value) {
    let values: Vec<_> = expected
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap() as f32)
        .collect();
    assert_eq!(
        [actual.x(), actual.y(), actual.width(), actual.height()].as_slice(),
        values
    );
}

/// 【OCR 表头测试】【两种语言布局】核对两种语言、五种宽度及复制失败的二十个实际表头，标题宽度和统计间距同时匹配，无参数、无返回值
#[test]
fn title_and_summary_match_the_actual_qt_header_in_both_languages() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-ui-language.json"
    ))
    .unwrap();
    let mut count = 0;
    for entry in fixture["languages"].as_array().unwrap() {
        let language = if entry["name"] == "zh" {
            Language::Chinese
        } else {
            Language::English
        };
        for case in entry["cases"].as_array().unwrap() {
            for state in case["states"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|state| state["name"].as_str().unwrap().starts_with("table"))
            {
                let widgets = &state["widgets"];
                let copy = &widgets["ocrTableCopyButton"]["rect"];
                let copy = rect(
                    copy[0].as_f64().unwrap() as f32,
                    copy[1].as_f64().unwrap() as f32,
                    copy[2].as_f64().unwrap() as f32,
                    copy[3].as_f64().unwrap() as f32,
                );
                let header = Header::new(copy, language);
                compare(header.title, &widgets["table:title"]["rect"]);
                compare(header.summary, &widgets["table:summary"]["rect"]);
                assert_eq!(header.title.right() + 6.0, header.summary.left());
                assert_eq!(header.summary.right() + 6.0, copy.left());
                count += 1;
            }
        }
    }
    assert_eq!(count, 20);
}
