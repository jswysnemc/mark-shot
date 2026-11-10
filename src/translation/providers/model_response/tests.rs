//! 【模型翻译测试】【真实原版解析】逐项核对只读 Qt 探针记录，覆盖失败文案、首候选与文本块组合

use super::*;

/// 【模型翻译测试】【协议记录】保留单块响应的原版记录，多块 Gemini 使用独立用例验证，不连接实际服务
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
        if provider == Provider::Gemini
            && body
                .pointer("/candidates/0/content/parts")
                .and_then(Value::as_array)
                .is_some_and(|parts| parts.len() > 1)
        {
            continue;
        }
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

/// 【模型翻译测试】【思考与多块】无参数，验证 Gemini 多块正文及 Responses 思考项不会污染译文
#[test]
fn reasoning_and_chunked_content_are_separated_from_translations() {
    let body = serde_json::json!({"candidates":[{"content":{"parts":[
        {"thought":true,"text":"private reasoning {not json}"},
        {"text":"{\"translations\":["}, {"text":"{\"id\":7,\"text\":\"译文\"}]}"}]}}]});
    assert_eq!(parse(Provider::Gemini, &body).unwrap()[&7], "译文");
    let body = serde_json::json!({"status":"completed","output":[
        {"type":"reasoning","summary":[{"text":"private"}]},
        {"type":"message","content":[{"type":"output_text","text":"{\"translations\":[{\"id\":7,\"text\":\"译文\"}]}"}]}]});
    assert_eq!(parse(Provider::Responses, &body).unwrap()[&7], "译文");
    let mut completed = body.clone();
    completed["error"] = Value::Null;
    assert_eq!(parse(Provider::Responses, &completed).unwrap()[&7], "译文");
    let mut incomplete = body.clone();
    incomplete["status"] = "incomplete".into();
    assert!(parse(Provider::Responses, &incomplete).is_err());
    let body = serde_json::json!({"choices":[{"message":{"content":"<think>{bad}</think>{\"translations\":[{\"id\":7,\"text\":\"译文\"}]}"}}]});
    assert_eq!(
        parse_options(Provider::Bilibili, &body, true).unwrap()[&7],
        "译文"
    );
    assert!(parse_options(Provider::Bilibili, &body, false).is_err());
}

/// 【模型翻译测试】【响应错误详情】无参数，Responses 字符串错误、错误代码及缺失描述都返回可诊断信息
#[test]
fn responses_errors_keep_string_messages_codes_and_missing_message_context() {
    for (error, expected) in [
        (serde_json::json!("model unavailable"), "model unavailable"),
        (
            serde_json::json!({"code":"model_not_found","message":"No such model"}),
            "model_not_found",
        ),
        (serde_json::json!({}), "without a message"),
    ] {
        let result = parse(Provider::Responses, &serde_json::json!({"error":error})).unwrap_err();
        assert_eq!(result.kind, ErrorKind::Api);
        assert!(result.detail.contains(expected));
    }
}
