//! 【模型目录测试】【协议约束】验证各协议的目录地址、分页和文本模型过滤

use super::*;
use serde_json::json;

/// 【模型目录测试】【路径与分页】无参数，验证生成地址转换和分页身份，返回空值
#[test]
fn catalog_paths_and_pagination_follow_protocols() {
    for (id, vendor, field, address, expected) in [
        (
            "bilibili",
            "bilibili",
            "baseUrl",
            "https://example.com/v1/chat/completions",
            "https://example.com/v1/models",
        ),
        (
            "openai-responses",
            "responses",
            "baseUrl",
            "https://example.com/prefix/v1/responses/",
            "https://example.com/prefix/v1/models",
        ),
        (
            "anthropic",
            "anthropic",
            "endpoint",
            "https://example.com",
            "https://example.com/v1/models",
        ),
        (
            "gemini",
            "gemini",
            "endpoint",
            "https://example.com/v1beta",
            "https://example.com/v1beta/models",
        ),
    ] {
        let root = json!({vendor:{field:address,"apiKey":"test-key"}});
        let Service::Llm(config) =
            resolve::service(&root, &format!("plugin:{id}"), &|_| None).unwrap()
        else {
            panic!()
        };
        assert_eq!(endpoint(&config).unwrap().as_str(), expected);
    }
    assert_eq!(page(Provider::Gemini, &json!({"models":[
        {"name":"models/translate","supportedGenerationMethods":["generateContent"]},
        {"name":"models/embed","supportedGenerationMethods":["embedContent"]}],"nextPageToken":"page+two"})).unwrap(),
        (vec!["translate".into()], Some("page+two".into())));
    assert_eq!(
        page(
            Provider::Anthropic,
            &json!({"data":[{"id":"claude-test"}],"has_more":true,"last_id":"claude-test"})
        )
        .unwrap()
        .1
        .as_deref(),
        Some("claude-test")
    );
    assert!(page(Provider::Anthropic, &json!({"data":[],"has_more":true})).is_err());
    assert!(page(Provider::OpenAi, &json!({"error":"invalid key"})).is_err());
}

/// 【模型目录测试】【实际分页】无参数，验证 Gemini 游标编码与第二页合并，返回空值
#[test]
fn gemini_directory_follows_encoded_cursor_over_http() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        time::Duration,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/v1beta", listener.local_addr().unwrap());
    let worker = std::thread::spawn(move || {
        let mut paths = Vec::new();
        for body in [
            json!({"models":[{"name":"models/first","supportedGenerationMethods":["generateContent"]}],"nextPageToken":"next+page/&"}),
            json!({"models":[{"name":"models/second","supportedGenerationMethods":["generateContent"]}]}),
        ] {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = String::new();
            let mut buffer = [0; 2048];
            while !request.contains("\r\n\r\n") {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                request.push_str(&String::from_utf8_lossy(&buffer[..count]));
            }
            paths.push(request.lines().next().unwrap().to_owned());
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("x-goog-api-key: fixture-key")
            );
            let body = body.to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
        paths
    });
    let Service::Llm(config) = resolve::service(
        &json!({"gemini":{"endpoint":endpoint,"apiKey":"fixture-key"}}),
        "plugin:gemini",
        &|_| None,
    )
    .unwrap() else {
        panic!()
    };
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(list(&config))
        .unwrap();
    assert_eq!(result, ["first", "second"]);
    assert_eq!(
        worker.join().unwrap(),
        [
            "GET /v1beta/models HTTP/1.1",
            "GET /v1beta/models?pageToken=next%2Bpage%2F%26 HTTP/1.1"
        ]
    );
}
