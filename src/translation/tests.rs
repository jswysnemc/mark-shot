//! 【翻译服务测试】【配置与映射】对照原版实际记录，测试不读取用户环境、配置或剪贴板

use super::*;
use serde_json::{Value, json};

/// 【翻译服务测试】【原版记录】返回只读探针冻结的协议记录，无参数，不连接原版运行窗口
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../tests/fixtures/qt-translation-services.json"
    ))
    .unwrap()
}

/// 【翻译服务测试】【原文身份】空行计入身份，确认原文只按行分段，未修改传入字符串，无参数、无返回值
#[test]
fn source_snapshot_preserves_blank_line_identity_and_caller_text() {
    let source = "  alpha\n\n 中文 \n beta  ";
    assert_eq!(
        source_segments(source),
        vec![
            Segment {
                id: 0,
                text: "alpha".into()
            },
            Segment {
                id: 2,
                text: "中文".into()
            },
            Segment {
                id: 3,
                text: "beta".into()
            }
        ]
    );
    assert_eq!(source, "  alpha\n\n 中文 \n beta  ");
    assert!(source_segments(" \r\n ").is_empty());
}

/// 【翻译服务测试】【原版解析】有效、包裹、重复身份和异常字段均与实际 Qt 解析一致，无参数、无返回值
#[test]
fn model_content_matches_original_parser_records() {
    for case in fixture()["content"].as_array().unwrap() {
        let actual = response::content(case["input"].as_str().unwrap());
        assert_eq!(actual.is_ok(), case["ok"].as_bool().unwrap(), "{case}");
        if let Ok(actual) = actual {
            assert_eq!(
                serde_json::to_value(actual).unwrap(),
                case["translations"],
                "{case}"
            );
        }
    }
}

/// 【翻译服务测试】【原版语言】界面自由输入与服务别名分别采用原版规则，未知语言不近似替换，无参数、无返回值
#[test]
fn languages_match_the_original_ui_and_provider_records() {
    for case in fixture()["languages"].as_array().unwrap() {
        let input = case["input"].as_str().unwrap();
        assert_eq!(
            language::canonical(input).unwrap_or_default(),
            case["key"].as_str().unwrap()
        );
        assert_eq!(language::value(input), case["ui_value"].as_str().unwrap());
    }
    for case in fixture()["aliases"].as_array().unwrap() {
        assert_eq!(
            language::canonical(case["input"].as_str().unwrap()).unwrap_or_default(),
            case["key"].as_str().unwrap(),
            "{case}"
        );
    }
    for case in fixture()["localized"].as_array().unwrap() {
        assert_eq!(
            language::value_for(
                case["input"].as_str().unwrap(),
                language::UiLanguage::Chinese
            ),
            case["ui_value"].as_str().unwrap(),
            "{case}"
        );
    }
    assert!(language::canonical("fre").is_none());
    assert_eq!(language::code(Provider::Baidu, "Japanese").unwrap(), "jp");
    assert_eq!(
        language::code(Provider::Youdao, "Traditional Chinese").unwrap(),
        "zh-CHT"
    );
    assert!(language::code(Provider::Tencent, "Cantonese").is_err());
    assert!(language::code(Provider::Baidu, "Klingon").is_err());
}

/// 【翻译服务测试】【厂商隔离】公共密钥不进入 Gemini 或 Anthropic，自动选择优先级稳定，显式服务不回退，无参数、无返回值
#[test]
fn provider_selection_and_credentials_stay_isolated() {
    let env = |_: &str| None;
    let config = json!({"apiKey":"shared-openai-secret","model":"shared-model", "gemini":{"apiKey":"gemini-secret"}});
    let automatic = resolve::service(&config, "auto", &env).unwrap();
    assert_eq!(automatic.provider(), Provider::OpenAi);
    let gemini = resolve::service(&config, "plugin:gemini", &env).unwrap();
    let resolve::Service::Llm(gemini) = gemini else {
        panic!("expected model service");
    };
    assert_eq!(gemini.key.0, "gemini-secret");
    assert_eq!(gemini.model, "gemini-3.5-flash-lite");
    assert!(gemini.temperature.is_none());
    assert!(resolve::service(&config, "plugin:anthropic", &env).is_err());
    let automatic = resolve::service(
        &json!({"gemini":{"apiKey":"gemini-secret"},"anthropic":{"apiKey":"anthropic-secret"}}),
        "auto",
        &env,
    )
    .unwrap();
    assert_eq!(automatic.provider(), Provider::Gemini);
    assert!(resolve::service(&config, "plugin:missing", &env).is_err());
}

/// 【翻译服务测试】【配置优先级】厂商字段优先于公共字段与环境，密钥环境名可配置，超时和请求正文校验独立，无参数、无返回值
#[test]
fn environment_fallback_and_vendor_overrides_match_original_precedence() {
    let env = |name: &str| (name == "PRIVATE_TEST_KEY").then(|| "environment-secret".into());
    let config = json!({"apiKeyEnv":"PRIVATE_TEST_KEY", "model":"shared-model", "openai":{"model":"vendor-model","timeoutMs":1}, "extraBody":{"model":"should-not-win"}});
    let service = resolve::service(&config, "plugin:openai-compatible", &env).unwrap();
    let resolve::Service::Llm(ref llm) = service else {
        panic!("expected model service");
    };
    assert_eq!(llm.key.0, "environment-secret");
    assert_eq!(llm.model, "vendor-model");
    assert_eq!(llm.timeout.as_millis(), 1000);
    let resolve::Service::Llm(builtin) = resolve::service(&config, "builtin", &env).unwrap() else {
        panic!("expected model service");
    };
    assert_eq!(builtin.model, "shared-model");
    assert_eq!(builtin.timeout.as_millis(), 60000);
    assert!(!format!("{service:?}").contains("environment-secret"));
    let mut error = Error::new(ErrorKind::Api, "echo environment-secret");
    service.redact(&mut error);
    assert_eq!(error.detail, "echo [redacted]");
    assert!(
        resolve::service(
            &json!({"apiKey":"test-key","extraBody":false}),
            "builtin",
            &env
        )
        .is_err()
    );
}

/// 【翻译服务测试】【回填边界】模型缺少译文时保留原文，批量服务条数不符拒绝结果，不允许错位更新，无参数、无返回值
#[test]
fn ordered_results_reject_misalignment_while_models_keep_original_fallback() {
    let source = source_segments("first\nsecond");
    let output = response::mapped(
        Provider::OpenAi,
        &source,
        &response::content("{\"translations\":[{\"id\":1,\"text\":\"译文\"}]}").unwrap(),
    )
    .unwrap();
    assert_eq!(output.text(), "first\n译文");
    assert!(response::ordered(Provider::Tencent, &source, vec!["only one".into()]).is_err());
    assert_eq!(
        response::ordered(Provider::Youdao, &source, vec!["".into(), "译文".into()])
            .unwrap()
            .text(),
        "first\n译文"
    );
}
