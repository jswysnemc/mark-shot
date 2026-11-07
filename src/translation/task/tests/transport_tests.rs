//! 【翻译传输测试】【实际边界】验证声明长度、分块超限与重定向隔离，所有地址仅使用回环测试服务

use super::*;

/// 【翻译传输测试】【实际任务错误】按 bytes 执行一次真实 HTTP 任务，返回错误，确认线程已经退出，不读取用户环境
fn error(bytes: Vec<u8>) -> Error {
    let server = Server::start_bytes(Some(bytes));
    let mut task = Task::start_with(
        &config(&server),
        "private source",
        "English",
        Path::new("/private/config"),
        WindowWake::detached().unwrap(),
        &|_| None,
    )
    .unwrap();
    let error = result(&mut task, Instant::now() + Duration::from_secs(3)).unwrap_err();
    server.closed.recv_timeout(Duration::from_secs(1)).unwrap();
    finished(&task);
    error
}

/// 【翻译传输测试】【长度声明】服务仅声明超大响应时立即拒绝，不等待正文或把截断当作 JSON 错误，无参数、无返回值
#[test]
fn declared_oversized_response_is_rejected_before_body_reading() {
    let error =
        error(b"HTTP/1.1 200 OK\r\nContent-Length: 4194305\r\nConnection: close\r\n\r\n".to_vec());
    assert_eq!(error.kind, ErrorKind::Protocol);
    assert_eq!(error.detail, "translation response exceeds four MiB");
}

/// 【翻译传输测试】【分块限制】没有总长度时仍拒绝超过四 MiB 的实际分块响应，不能因为未知长度继续缓存，无参数、无返回值
#[test]
fn chunked_response_without_content_length_has_the_same_memory_limit() {
    let mut bytes =
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".to_vec();
    for _ in 0..5 {
        bytes.extend_from_slice(b"100000\r\n");
        bytes.extend(std::iter::repeat_n(b'x', 1024 * 1024));
        bytes.extend_from_slice(b"\r\n");
    }
    bytes.extend_from_slice(b"0\r\n\r\n");
    let error = error(bytes);
    assert_eq!(error.kind, ErrorKind::Protocol);
    assert_eq!(error.detail, "translation response exceeds four MiB");
}

/// 【翻译传输测试】【重定向隔离】307 指向另一个真实回环监听口时返回 HTTP 失败，目标口不收到凭据或原文，无参数、无返回值
#[test]
fn redirect_never_sends_source_or_credentials_to_the_second_endpoint() {
    let destination = TcpListener::bind("127.0.0.1:0").unwrap();
    destination.set_nonblocking(true).unwrap();
    let response = format!(
        "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://{}/untrusted\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        destination.local_addr().unwrap()
    );
    let error = error(response.into_bytes());
    assert_eq!(error.kind, ErrorKind::Http);
    assert!(error.detail.contains("http 307"));
    assert_eq!(
        destination.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
