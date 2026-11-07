use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
};

/// 【上传测试】【HTTP接收】在本机随机端口接收完整实际请求并返回 response，返回端点和接收线程；不使用真实服务
fn server(response: Vec<u8>) -> (String, std::thread::JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let worker = std::thread::spawn(move || {
        let (mut client, _) = listener.accept().unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(4)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut chunk = [0; 8192];
        loop {
            let count = client.read(&mut chunk).unwrap();
            assert!(
                count > 0,
                "HTTP client closed before uploading complete image"
            );
            bytes.extend_from_slice(&chunk[..count]);
            if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
                let length: usize = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length: "))
                    .unwrap()
                    .parse()
                    .unwrap();
                if bytes.len() >= end + 4 + length {
                    break;
                }
            }
        }
        let _ = client.write_all(&response);
        bytes
    });
    (format!("http://{address}/upload"), worker)
}

/// 【上传测试】【HTTP回包】按 status 和 body 构造完整 HTTP 响应，返回字节；长度使用实际字节数
fn response(status: &str, body: &[u8]) -> Vec<u8> {
    [
        format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes(),
        body.to_vec(),
    ]
    .concat()
}

/// 【上传测试】【实际上传】验证字段、鉴权、额外请求头、完整 PNG 和嵌套 URL，通过真实 TCP 请求完成
#[test]
fn actual_http_upload_preserves_multipart_and_provider_response() {
    let (endpoint, worker) = server(response("200 OK", br#"{"data":[{"link":"https://images.example/result.png","remove":"https://images.example/delete"}]}"#));
    let image = include_bytes!("../fixtures/qt-export-image/case-17-source.png").to_vec();
    let config = Upload::from_value(&serde_json::json!({"env":{
        "MARK_SHOT_UPLOAD_URL":endpoint,"MARK_SHOT_UPLOAD_FIELD":"file","MARK_SHOT_UPLOAD_API_KEY":"local-only-token",
        "MARK_SHOT_UPLOAD_HEADER_X-Application":"test","MARK_SHOT_UPLOAD_FIELD_album":"local",
        "MARK_SHOT_UPLOAD_URL_PATH":"data.0.link","MARK_SHOT_UPLOAD_DELETE_URL_PATH":"data.0.remove"}}));
    let result = run(config.clone(), image.clone()).unwrap();
    assert_eq!(result.url, "https://images.example/result.png");
    assert_eq!(result.delete_url, "https://images.example/delete");
    let bytes = worker.join().unwrap();
    let end = bytes
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .unwrap();
    let headers = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
    assert!(headers.starts_with("post /upload http/1.1"));
    assert!(
        headers.contains("authorization: bearer local-only-token")
            && headers.contains("x-application: test")
    );
    assert_eq!(
        &bytes[end + 4..],
        upload::request::multipart(&config, &image, "screenshot.png").unwrap()
    );
}

/// 【上传测试】【失败边界】验证实际 HTTP 错误、无长度超限响应、无 URL 和取消；错误不包含端点凭据或响应正文
#[test]
fn http_errors_invalid_results_and_unbounded_responses_fail() {
    for (response, expected) in [
        (
            response("403 Forbidden", b"private-provider-secret"),
            upload::ErrorKind::Http,
        ),
        (
            response("200 OK", br#"{"success":true}"#),
            upload::ErrorKind::Protocol,
        ),
        (
            [
                b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec(),
                vec![b'x'; upload::MAX_RESPONSE + 10],
            ]
            .concat(),
            upload::ErrorKind::Protocol,
        ),
    ] {
        let (endpoint, worker) = server(response);
        let config = Upload::from_value(
            &serde_json::json!({"env":{"MARK_SHOT_UPLOAD_URL":format!("{endpoint}?token=private-provider-secret")}}),
        );
        let error = run(config, vec![1, 2, 3]).unwrap_err();
        assert_eq!(error.kind, expected);
        assert!(!error.to_string().contains("private-provider-secret"));
        worker.join().unwrap();
    }
}

/// 【上传测试】【网络取消】真实服务器收到图片后不返回正文，主动取消应立即通知并回收工作线程
#[test]
fn cancel_actual_pending_http_without_waiting_for_network_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/upload", listener.local_addr().unwrap());
    let (accepted, receipt) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let (_client, _) = listener.accept().unwrap();
        accepted.send(()).unwrap();
        released.recv_timeout(Duration::from_secs(4)).unwrap();
    });
    let config = Upload::from_value(&serde_json::json!({"env":{"MARK_SHOT_UPLOAD_URL":endpoint}}));
    let (done, received) = mpsc::channel();
    let mut task = Task::start(config, vec![1, 2, 3], move || {
        let _ = done.send(());
    })
    .unwrap();
    receipt.recv_timeout(Duration::from_secs(3)).unwrap();
    let start = std::time::Instant::now();
    task.cancel();
    received.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(
        task.take_result().unwrap().unwrap_err().kind,
        upload::ErrorKind::Canceled
    );
    task.shutdown();
    assert!(start.elapsed() < Duration::from_secs(1));
    release.send(()).unwrap();
    worker.join().unwrap();
}

/// 【上传测试】【实际重定向】图床返回三百零三时继续取得最终 URL，保持原版辅助脚本的常见重定向流程
#[test]
fn follows_actual_provider_redirect_to_final_url() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let worker = std::thread::spawn(move || {
        for redirected in [false, true] {
            let (mut client, _) = listener.accept().unwrap();
            client
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0; 4096];
            loop {
                let count = client.read(&mut chunk).unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&chunk[..count]);
                if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
                    let length = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .map(|value| value.parse::<usize>().unwrap())
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            if redirected {
                assert!(bytes.starts_with(b"GET /result HTTP/1.1"));
                client
                    .write_all(&response("200 OK", b"https://images.example/redirected"))
                    .unwrap();
            } else {
                assert!(bytes.starts_with(b"POST /upload HTTP/1.1"));
                client.write_all(format!("HTTP/1.1 303 See Other\r\nLocation: http://{address}/result\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).unwrap();
            }
        }
    });
    let config = Upload::from_value(
        &serde_json::json!({"env":{"MARK_SHOT_UPLOAD_URL":format!("http://{address}/upload")}}),
    );
    assert_eq!(
        run(config, vec![1, 2, 3]).unwrap().url,
        "https://images.example/redirected"
    );
    worker.join().unwrap();
}
