//! 【翻译协议测试】【服务边界】核对签名、请求和降级，不通过实际外部服务消耗凭据或发送原文

use super::super::{
    Error, ErrorKind,
    http::{Request, Transport},
    resolve, response,
};
use super::*;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::{collections::VecDeque, sync::Mutex};

struct HttpFixture {
    replies: Mutex<VecDeque<Result<Value>>>,
    requests: Mutex<Vec<Request>>,
}

impl HttpFixture {
    /// 【翻译协议测试】【回复队列】按 replies 返回固定传输替身，不建立网络连接或读取用户环境
    fn new(replies: Vec<Value>) -> Self {
        Self {
            replies: Mutex::new(replies.into_iter().map(Ok).collect()),
            requests: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl Transport for HttpFixture {
    /// 【翻译协议测试】【捕获请求】保存 request 并返回下一条固定回复，队列为空时失败，不伪造多余请求成功
    async fn send(&self, request: Request) -> Result<Value> {
        self.requests.lock().unwrap().push(request);
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err(Error::new(ErrorKind::Protocol, "unexpected extra request")))
    }
}

/// 【翻译协议测试】【真实规则执行】按 config、text、target 和 fixture 运行实际服务代码，返回结果，不使用进程环境
fn run(
    config: Value,
    provider: &str,
    text: &str,
    target: &str,
    fixture: &HttpFixture,
) -> Result<Output> {
    let service = resolve::service(&config, provider, &|_| None)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(translate(
            &service,
            &super::super::source_segments(text),
            target,
            fixture,
        ))
}

/// 【翻译协议测试】【原版签名】腾讯、百度、有道签名逐字节对照真实 Qt 向量，包含 UTC 跨日及 UTF-16 代理对，无参数、无返回值
#[test]
fn signing_matches_original_qt_vectors() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/qt-translation-services.json"
    ))
    .unwrap();
    for row in fixture["baidu"].as_array().unwrap() {
        assert_eq!(
            baidu::signature(
                "test-app",
                row["query"].as_str().unwrap(),
                "test-salt",
                "test-secret"
            ),
            row["signature"].as_str().unwrap()
        );
    }
    for row in fixture["youdao"].as_array().unwrap() {
        let query = row["query"].as_str().unwrap();
        assert_eq!(
            hex(youdao::sign_input(query).as_bytes()),
            row["input_utf8_hex"].as_str().unwrap(),
            "{row}"
        );
        assert_eq!(
            youdao::signature("test-app", query, "test-salt", "1551113065", "test-secret"),
            row["signature"].as_str().unwrap(),
            "{row}"
        );
    }
    for row in fixture["tc3"].as_array().unwrap() {
        let body = row["body"].as_str().unwrap().as_bytes();
        assert_eq!(
            tc3::canonical("TMT.TencentCloudApi.com", body),
            row["canonical"].as_str().unwrap()
        );
        assert_eq!(
            tc3::authorization(
                "test-id",
                "test-secret",
                "TMT.TencentCloudApi.com",
                body,
                row["timestamp"].as_i64().unwrap()
            )
            .unwrap(),
            row["authorization"].as_str().unwrap()
        );
    }
}

/// 【翻译协议测试】【模型字段】三个模型服务分别使用正确正文和鉴权，Anthropic 默认不下发温度，extraBody 不覆盖必要字段，无参数、无返回值
#[test]
fn llm_payloads_and_response_parts_follow_each_provider_protocol() {
    for (provider, settings, reply, endpoint, auth) in [
        (
            "builtin",
            json!({"apiKey":"test-key","apiBase":"http://localhost/v1/chat/completions/","extraBody":{"model":"wrong","stream":false}}),
            json!({"choices":[{"message":{"content":"{\"translations\":[{\"id\":0,\"text\":\"translated\"}]}"}}]}),
            "http://localhost/v1/chat/completions",
            "Authorization",
        ),
        (
            "plugin:gemini",
            json!({"gemini":{"apiKey":"test-key","endpoint":"http://localhost","model":"models/test-model","thinkingLevel":"minimal"}}),
            json!({"candidates":[{"content":{"parts":[{"text":"{\"translations\":[{\"id\":0,\"text\":\"translated\"}]}"}]}}]}),
            "http://localhost/v1beta/models/test-model:generateContent",
            "x-goog-api-key",
        ),
        (
            "plugin:anthropic",
            json!({"anthropic":{"apiKey":"test-key","endpoint":"http://localhost"}}),
            json!({"content":[{"type":"thinking","text":"ignored"},{"type":"text","text":"{\"translations\":"},{"type":"text","text":"[{\"id\":0,\"text\":\"translated\"}]}"}]}),
            "http://localhost/v1/messages",
            "x-api-key",
        ),
    ] {
        let http = HttpFixture::new(vec![reply]);
        assert_eq!(
            run(settings, provider, "source", "English", &http)
                .unwrap()
                .text(),
            "translated"
        );
        let requests = http.requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].endpoint, endpoint);
        assert!(requests[0].headers.iter().any(|(key, _)| key == auth));
        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        if provider == "builtin" {
            assert_eq!(body["model"], "gpt-4o-mini");
            assert_eq!(body["stream"], false);
        }
        if provider == "plugin:anthropic" {
            assert!(body.get("temperature").is_none());
            assert!(
                requests[0]
                    .headers
                    .iter()
                    .any(|(key, value)| key == "anthropic-version" && value == "2023-06-01")
            );
        }
        if provider == "plugin:gemini" {
            assert!(body["generationConfig"].get("temperature").is_none());
            assert_eq!(
                body["generationConfig"]["thinkingConfig"]["thinkingLevel"],
                "MINIMAL"
            );
        }
    }
}

/// 【翻译协议测试】【腾讯降级】下线错误只探测一次批量接口，后续批次全部调用单段，错误条数不得继续回填，无参数、无返回值
#[test]
fn tencent_action_offline_uses_only_individual_requests_after_first_failure() {
    let text = format!("{}\n{}", "a".repeat(1500), "b".repeat(1500));
    let http = HttpFixture::new(vec![
        json!({"Response":{"Error":{"Code":"ActionOffline.TextTranslateBatch","Message":"offline"}}}),
        json!({"Response":{"TargetText":"first"}}),
        json!({"Response":{"TargetText":"second"}}),
    ]);
    let settings = json!({"tencent":{"secretId":"test-id","secretKey":"test-secret","endpoint":"http://localhost:8123/ignored"}});
    assert_eq!(
        run(
            settings.clone(),
            "plugin:tencent-tmt",
            &text,
            "Simplified Chinese",
            &http
        )
        .unwrap()
        .text(),
        "first\nsecond"
    );
    let requests = http.requests.lock().unwrap();
    let actions: Vec<_> = requests
        .iter()
        .map(|request| {
            request
                .headers
                .iter()
                .find(|(key, _)| key == "X-TC-Action")
                .unwrap()
                .1
                .as_str()
        })
        .collect();
    assert_eq!(
        actions,
        ["TextTranslateBatch", "TextTranslate", "TextTranslate"]
    );
    for request in requests.iter() {
        assert_eq!(request.endpoint, "http://localhost:8123/");
        let time = request
            .headers
            .iter()
            .find(|(key, _)| key == "X-TC-Timestamp")
            .unwrap()
            .1
            .parse()
            .unwrap();
        let auth = request
            .headers
            .iter()
            .find(|(key, _)| key == "Authorization")
            .unwrap();
        assert_eq!(
            auth.1,
            tc3::authorization(
                "test-id",
                "test-secret",
                "localhost:8123",
                &request.body,
                time
            )
            .unwrap()
        );
    }
    let mismatch = HttpFixture::new(vec![json!({"Response":{"TargetTextList":[]}})]);
    assert!(
        run(
            settings,
            "plugin:tencent-tmt",
            "source",
            "English",
            &mismatch
        )
        .is_err()
    );
}

/// 【翻译协议测试】【表单与错误】百度加号编码、有道多个 q 字段及数字错误码均走真实实现，秘密回显必须脱敏，无参数、无返回值
#[test]
fn form_encoding_counts_and_provider_errors_are_checked() {
    let baidu = HttpFixture::new(vec![json!({"trans_result":[{"dst":"one"},{"dst":"two"}]})]);
    let config = json!({"baidu":{"appId":"test-id","appKey":"test-secret"}});
    assert_eq!(
        run(
            config.clone(),
            "plugin:baidu-fanyi",
            "A+B %\nC D",
            "Japanese",
            &baidu
        )
        .unwrap()
        .text(),
        "one\ntwo"
    );
    let body = String::from_utf8(baidu.requests.lock().unwrap()[0].body.clone()).unwrap();
    assert!(body.starts_with("q=A%2BB+%25%0AC+D&from=auto&to=jp&appid=test-id&salt="));
    let error = HttpFixture::new(vec![
        json!({"error_code":54001,"error_msg":"echo test-secret"}),
    ]);
    let error = run(config, "plugin:baidu-fanyi", "source", "English", &error).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Api);
    assert!(!error.detail.contains("test-secret"));
    let youdao = HttpFixture::new(vec![json!({"errorCode":0,"translation":["one","two"]})]);
    assert_eq!(
        run(
            json!({"youdao":{"appKey":"test-id","appSecret":"test-secret"}}),
            "plugin:youdao-nmt",
            "A B+\nC",
            "English",
            &youdao
        )
        .unwrap()
        .text(),
        "one\ntwo"
    );
    let body = String::from_utf8(youdao.requests.lock().unwrap()[0].body.clone()).unwrap();
    assert!(body.starts_with("q=A%20B%2B&q=C&from=auto&to=en&appKey=test-id&salt="));
    assert!(body.contains("&signType=v3&curtime="));
}

/// 【翻译协议测试】【预算语义】批次按 UTF-16 而非 UTF-8 或 Unicode 标量计数，单段超预算不丢失身份，无参数、无返回值
#[test]
fn batching_uses_utf16_budget_without_splitting_source_segments() {
    let source = vec![
        Segment {
            id: 0,
            text: "\u{1f680}".repeat(3),
        },
        Segment {
            id: 1,
            text: "abcdef".into(),
        },
    ];
    let chunks = batches(&source, 6);
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0][0].id, 0);
    assert_eq!(chunks[1][0].id, 1);
    assert_eq!(
        response::prompt(&source, "")["target_language"],
        "Simplified Chinese"
    );
}

/// 【翻译协议测试】【表单特殊符号】星号、波浪线和空格的编码逐字节匹配真实 Qt，不混用 HTML 表单与 URL 编码规则，无参数、无返回值
#[test]
fn form_special_characters_match_actual_qt_encoding() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/qt-translation-services.json"
    ))
    .unwrap();
    let form = &fixture["form"];
    let plain = form["plain"].as_str().unwrap();
    assert_eq!(encode(plain, true), form["baidu"].as_str().unwrap());
    assert_eq!(encode(plain, false), form["youdao"].as_str().unwrap());
}

/// 【翻译协议测试】【原文回退】空译文保留未压平的原文，批次压平只影响发送，不得改变制表符或原文换行，无参数、无返回值
#[test]
fn partial_cloud_translations_preserve_original_tabs_not_flattened_request_text() {
    for (provider, config, body) in [
        (
            "plugin:tencent-tmt",
            json!({"tencent":{"secretId":"test-id","secretKey":"test-key"}}),
            json!({"Response":{"TargetTextList":["", "译文"]}}),
        ),
        (
            "plugin:baidu-fanyi",
            json!({"baidu":{"appId":"test-id","appKey":"test-key"}}),
            json!({"trans_result":[{"dst":""}, {"dst":"译文"}]}),
        ),
        (
            "plugin:youdao-nmt",
            json!({"youdao":{"appKey":"test-id","appSecret":"test-key"}}),
            json!({"errorCode":0,"translation":["", "译文"]}),
        ),
    ] {
        let http = HttpFixture::new(vec![body]);
        assert_eq!(
            run(config, provider, "A\tB\nsecond", "English", &http)
                .unwrap()
                .text(),
            "A\tB\n译文",
            "{provider}"
        );
    }
}

/// 【翻译协议测试】【拒绝空译文】无参数，验证云服务全空响应返回协议错误，不把原文作为成功结果
#[test]
fn empty_cloud_translations_are_rejected() {
    for (provider, config, body) in [
        (
            "plugin:tencent-tmt",
            json!({"tencent":{"secretId":"test-id","secretKey":"test-key"}}),
            json!({"Response":{"TargetTextList":[" "]}}),
        ),
        (
            "plugin:baidu-fanyi",
            json!({"baidu":{"appId":"test-id","appKey":"test-key"}}),
            json!({"trans_result":[{"dst":" "}]}),
        ),
        (
            "plugin:youdao-nmt",
            json!({"youdao":{"appKey":"test-id","appSecret":"test-key"}}),
            json!({"errorCode":0,"translation":[" "]}),
        ),
    ] {
        let http = HttpFixture::new(vec![body]);
        let error = run(config, provider, "original", "English", &http).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Protocol, "{provider}");
    }
}
