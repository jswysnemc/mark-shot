//! 【模型目录测试】【真实网络】验证真实 GET、分页和取消，服务仅监听私有回环地址

use super::*;
use serde_json::json;

/// 【翻译服务测试】【命令选择】无参数，显式命令模式未填写模板时必须报错，不能把原文发给默认网络服务
#[test]
fn empty_explicit_command_does_not_fall_back_to_network() {
    let config = Translation {
        provider: "command".into(),
        ..Translation::default()
    };
    let result = Task::start_with(
        &config,
        "source",
        "Chinese",
        Path::new("/private/config"),
        WindowWake::detached().unwrap(),
        &|_| None,
    );
    assert!(matches!(
        result,
        Err(Error {
            kind: ErrorKind::Configuration,
            ..
        })
    ));
}

/// 【模型目录测试】【查询完成】无参数，确认原生客户端发送 GET、应用标识与鉴权且没有 Origin，返回空值
#[test]
fn model_query_uses_get_and_native_headers() {
    let server = Server::start(Some((
        200,
        r#"{"data":[{"id":"model-z"},{"id":"model-a"},{"id":"model-z"}]}"#,
    )));
    let root = json!({"bilibili":{"baseUrl":server.endpoint,"apiKey":"test-key"}});
    let task = crate::translation::models::Task::start(&root, "plugin:bilibili", || {}).unwrap();
    let request = server
        .seen
        .recv_timeout(Duration::from_secs(2))
        .unwrap()
        .to_ascii_lowercase();
    assert!(request.starts_with("get /v1/models http/1.1"));
    assert!(request.contains("user-agent: mark-shot/"));
    assert!(request.contains("authorization: bearer test-key"));
    assert!(!request.contains("origin:"));
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(result) = task.finished() {
            assert_eq!(result.unwrap(), ["model-a", "model-z"]);
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// 【模型目录测试】【取消连接】无参数，查询正在等待响应时销毁任务必须关闭实际连接，返回空值
#[test]
fn cancel_model_query_releases_socket() {
    let server = Server::start(None);
    let root = json!({"bilibili":{"baseUrl":server.endpoint}});
    let task = crate::translation::models::Task::start(&root, "plugin:bilibili", || {}).unwrap();
    server.seen.recv_timeout(Duration::from_secs(2)).unwrap();
    drop(task);
    server.closed.recv_timeout(Duration::from_secs(2)).unwrap();
}

/// 【翻译服务测试】【新增协议】无参数，验证哔哩哔哩与 Responses 经真实 HTTP 完成翻译，返回空值
#[test]
fn new_providers_complete_through_native_transport() {
    for (id, vendor, endpoint, response) in [
        (
            "bilibili",
            "bilibili",
            "/v1/chat/completions",
            json!({"choices":[{"message":{"content":"{\"translations\":[{\"id\":0,\"text\":\"译文\"}]}"}}]}),
        ),
        (
            "openai-responses",
            "responses",
            "/v1/responses",
            json!({"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"{\"translations\":[{\"id\":0,\"text\":\"译文\"}]}"}]}]}),
        ),
    ] {
        let server = Server::start(Some((200, &response.to_string())));
        let config = serde_json::from_value(json!({"provider":format!("plugin:{id}"),vendor:{"baseUrl":server.endpoint,"apiKey":"test-key"}})).unwrap();
        let mut task = Task::start_with(
            &config,
            "source",
            "Chinese",
            Path::new("/private/config"),
            WindowWake::detached().unwrap(),
            &|_| None,
        )
        .unwrap();
        let request = server.seen.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(request.starts_with(&format!("POST {endpoint} HTTP/1.1")));
        assert_eq!(
            result(&mut task, Instant::now() + Duration::from_secs(2))
                .unwrap()
                .text(),
            "译文"
        );
        finished(&task);
    }
}
