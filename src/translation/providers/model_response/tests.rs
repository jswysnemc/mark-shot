//! 【模型翻译测试】【真实原版解析】逐项核对只读 Qt 探针记录，覆盖失败文案、首候选与文本块组合

use super::*;

/// 【模型翻译测试】【协议记录】全部成功映射和失败文案匹配原版函数，不连接实际服务，无参数、无返回值
#[test]
fn model_responses_match_actual_qt_parser_records() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-translation-model-responses.json"
    ))
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let provider = match case["provider"].as_str().unwrap() {
            "gemini" => Provider::Gemini,
            _ => Provider::Anthropic,
        };
        let body: Value = serde_json::from_str(case["body"].as_str().unwrap()).unwrap();
        let actual = parse(provider, &body);
        assert_eq!(actual.is_ok(), case["ok"].as_bool().unwrap(), "{case}");
        match actual {
            Ok(translations) => assert_eq!(
                serde_json::to_value(translations).unwrap(),
                case["translations"],
                "{case}"
            ),
            Err(error) => assert_eq!(error.detail, case["error"].as_str().unwrap(), "{case}"),
        }
    }
}
