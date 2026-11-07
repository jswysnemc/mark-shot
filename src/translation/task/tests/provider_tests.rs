//! 【翻译服务测试】【实际 HTTP】六类服务均通过私有回环服务验证真实传输、鉴权与协议，不访问外部接入点

use super::*;
use serde_json::{Value, json};

/// 【翻译服务测试】【厂商配置】按 provider 和 endpoint 返回显式服务配置，所有凭据固定为测试值，不使用用户环境
fn configuration(provider: Provider, endpoint: &str) -> Translation {
    let mut root = match provider {
        Provider::OpenAi => {
            json!({"openai":{"apiKey":"private-test-key","apiBase":endpoint,"model":"test-model"}})
        }
        Provider::Gemini => {
            json!({"gemini":{"apiKey":"private-test-key","endpoint":endpoint,"model":"test-model"}})
        }
        Provider::Anthropic => {
            json!({"anthropic":{"apiKey":"private-test-key","endpoint":endpoint,"model":"test-model"}})
        }
        Provider::Tencent => {
            json!({"tencent":{"secretId":"private-test-id","secretKey":"private-test-key","endpoint":endpoint}})
        }
        Provider::Baidu => {
            json!({"baidu":{"appId":"private-test-id","appKey":"private-test-key","endpoint":endpoint}})
        }
        Provider::Youdao => {
            json!({"youdao":{"appKey":"private-test-id","appSecret":"private-test-key","endpoint":endpoint}})
        }
        _ => panic!("expected native HTTP provider"),
    };
    root["provider"] = json!(format!("plugin:{}", provider.id()));
    serde_json::from_value(root).unwrap()
}

/// 【翻译服务测试】【成功协议】按 provider 返回两段真实服务回复，包含空译文回退，不创建模型生成内容
fn response(provider: Provider) -> String {
    let content = r#"{"translations":[{"id":0,"text":"translated"},{"id":2,"text":""}]}"#;
    match provider {
        Provider::OpenAi => json!({"choices":[{"message":{"content":content}}]}),
        Provider::Gemini => json!({"candidates":[{"content":{"parts":[{"text":content}]}}]}),
        Provider::Anthropic => json!({"content":[{"type":"text","text":content}]}),
        Provider::Tencent => json!({"Response":{"TargetTextList":["translated",""]}}),
        Provider::Baidu => json!({"trans_result":[{"dst":"translated"},{"dst":""}]}),
        Provider::Youdao => json!({"errorCode":"0","translation":["translated",""]}),
        _ => panic!("expected native HTTP provider"),
    }
    .to_string()
}

/// 【翻译服务测试】【请求字段】按 request 返回原始首行、首部和正文，各项借用捕获字节，不再次编码请求
fn fields(request: &str) -> (&str, &str, &str) {
    let (headers, body) = request.split_once("\r\n\r\n").unwrap();
    (headers.lines().next().unwrap(), headers, body)
}

/// 【翻译服务测试】【六类实际完成】逐项验证完整任务的真实 HTTP、服务身份、空译文原文回退与线程退出，无参数、无返回值
#[test]
fn every_native_provider_completes_over_actual_loopback_http() {
    for provider in [
        Provider::OpenAi,
        Provider::Gemini,
        Provider::Anthropic,
        Provider::Tencent,
        Provider::Baidu,
        Provider::Youdao,
    ] {
        let response = response(provider);
        let server = Server::start(Some((200, &response)));
        let config = configuration(provider, &server.endpoint);
        let source = " A+B %~*\n\nC\tD ";
        let mut task = Task::start_with(
            &config,
            source,
            "English",
            Path::new("/private/config"),
            WindowWake::detached().unwrap(),
            &|_| None,
        )
        .unwrap();
        assert_eq!(task.provider, provider);
        let request = server.seen.recv_timeout(Duration::from_secs(2)).unwrap();
        let (line, headers, body) = fields(&request);
        let headers = headers.to_ascii_lowercase();
        match provider {
            Provider::OpenAi | Provider::Gemini | Provider::Anthropic => {
                let (path, authentication) = match provider {
                    Provider::OpenAi => (
                        "/v1/chat/completions",
                        "authorization: bearer private-test-key",
                    ),
                    Provider::Gemini => (
                        "/v1/models/test-model:generateContent",
                        "x-goog-api-key: private-test-key",
                    ),
                    _ => ("/v1/messages", "x-api-key: private-test-key"),
                };
                assert_eq!(line, format!("POST {path} HTTP/1.1"));
                assert!(headers.contains(authentication));
                let payload: Value = serde_json::from_str(body).unwrap();
                let prompt = match provider {
                    Provider::OpenAi => &payload["messages"][1]["content"],
                    Provider::Gemini => &payload["contents"][0]["parts"][0]["text"],
                    _ => &payload["messages"][0]["content"],
                };
                let prompt: Value = serde_json::from_str(prompt.as_str().unwrap()).unwrap();
                assert_eq!(prompt["segments"][1], json!({"id":2,"text":"C\tD"}));
                if provider == Provider::Anthropic {
                    assert!(headers.contains("anthropic-version: 2023-06-01"));
                }
            }
            Provider::Tencent => {
                assert_eq!(line, "POST / HTTP/1.1");
                assert!(headers.contains("x-tc-action: texttranslatebatch"));
                assert!(
                    headers.contains("authorization: tc3-hmac-sha256 credential=private-test-id/")
                );
                let payload: Value = serde_json::from_str(body).unwrap();
                assert_eq!(payload["SourceTextList"], json!(["A+B %~*", "C D"]));
                assert_eq!(payload["Target"], "en");
            }
            Provider::Baidu | Provider::Youdao => {
                assert_eq!(line, "POST /v1 HTTP/1.1");
                assert!(headers.contains("content-type: application/x-www-form-urlencoded"));
                let pairs: Vec<_> = url::form_urlencoded::parse(body.as_bytes()).collect();
                let queries: Vec<_> = pairs
                    .iter()
                    .filter(|(key, _)| key == "q")
                    .map(|(_, value)| value.as_ref())
                    .collect();
                if provider == Provider::Baidu {
                    assert_eq!(queries, ["A+B %~*\nC D"]);
                    assert!(body.contains("A%2BB+%25~%2A"));
                } else {
                    assert_eq!(queries, ["A+B %~*", "C D"]);
                    assert!(body.contains("A%2BB%20%25~%2A"));
                }
            }
            _ => unreachable!(),
        }
        let output = result(&mut task, Instant::now() + Duration::from_secs(2)).unwrap();
        assert_eq!(output.provider, provider);
        assert_eq!(output.text(), "translated\nC\tD", "{provider:?}");
        assert_eq!(output.segments[1].id, 2);
        assert_eq!(source, " A+B %~*\n\nC\tD ");
        server.closed.recv_timeout(Duration::from_secs(1)).unwrap();
        finished(&task);
    }
}

/// 【翻译服务测试】【真实业务错误】各厂商成功 HTTP 中的业务错误必须失败并脱敏，不把错误结果回填为原文，无参数、无返回值
#[test]
fn every_native_provider_reports_and_redacts_actual_http_business_errors() {
    for provider in [
        Provider::OpenAi,
        Provider::Gemini,
        Provider::Anthropic,
        Provider::Tencent,
        Provider::Baidu,
        Provider::Youdao,
    ] {
        let body = match provider {
            Provider::Tencent => json!({"Response":{"Error":{"Code":"AuthFailure","Message":"echo private-test-key private-test-id"},"RequestId":"test-request"}}),
            Provider::Baidu => json!({"error_code":"54001","error_msg":"echo private-test-key private-test-id"}),
            Provider::Youdao => json!({"errorCode":"202","requestId":"echo private-test-key private-test-id"}),
            _ => json!({"error":{"message":"echo private-test-key"}}),
        }.to_string();
        let server = Server::start(Some((200, &body)));
        let mut task = Task::start_with(
            &configuration(provider, &server.endpoint),
            "source",
            "English",
            Path::new("/private/config"),
            WindowWake::detached().unwrap(),
            &|_| None,
        )
        .unwrap();
        let error = result(&mut task, Instant::now() + Duration::from_secs(2)).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Api);
        assert_eq!(error.provider, Some(provider));
        assert!(
            !error.detail.contains("private-test-key") && !error.detail.contains("private-test-id"),
            "{provider:?}"
        );
        assert!(error.detail.contains("[redacted]"));
        server.closed.recv_timeout(Duration::from_secs(1)).unwrap();
        finished(&task);
    }
}

/// 【翻译服务测试】【六类真实取消】服务确认收到原文后取消，各类任务均关闭实际连接并退出线程，不保留旧任务通知，无参数、无返回值
#[test]
fn every_native_provider_cancellation_closes_its_actual_connection() {
    for provider in [
        Provider::OpenAi,
        Provider::Gemini,
        Provider::Anthropic,
        Provider::Tencent,
        Provider::Baidu,
        Provider::Youdao,
    ] {
        let server = Server::start(None);
        let mut task = Task::start_with(
            &configuration(provider, &server.endpoint),
            "source",
            "English",
            Path::new("/private/config"),
            WindowWake::detached().unwrap(),
            &|_| None,
        )
        .unwrap();
        server.seen.recv_timeout(Duration::from_secs(2)).unwrap();
        task.cancel();
        assert_eq!(
            result(&mut task, Instant::now() + Duration::from_secs(1))
                .unwrap_err()
                .kind,
            ErrorKind::Canceled
        );
        server.closed.recv_timeout(Duration::from_secs(1)).unwrap();
        finished(&task);
    }
}
