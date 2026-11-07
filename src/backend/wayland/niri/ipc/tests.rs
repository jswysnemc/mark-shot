//! 【niri 回归】【IPC 边界】身份、换行传输、窗口稳定性与输出缩放

use super::*;
use serde_json::json;

/// 【niri 回归】【测试身份】按 socket 返回本进程内核身份，供捕获测试使用
pub(crate) fn peer(socket: &impl AsFd) -> Peer {
    Peer::read(socket).unwrap()
}

/// 【niri 回归】【测试连接】创建本进程内 socket 对，无参数；返回客户端与模拟服务端，不连接桌面
fn pair() -> (Client, UnixStream) {
    let (client, server) = UnixStream::pair().unwrap();
    let peer = Peer::read(&client).unwrap();
    (
        Client {
            reader: BufReader::new(client),
            path: PathBuf::new(),
            peer,
        },
        server,
    )
}

/// 【niri 回归】【合并事件】同一批传输包含回复与多个事件，所有换行记录均须保留，无参数、无返回值
#[tokio::test]
async fn coalesced_event_records_are_not_discarded() {
    let (mut client, mut server) = pair();
    server.write_all(b"{\"Ok\":\"Handled\"}\n{\"WindowClosed\":{\"id\":2}}\n{\"ScreenshotCaptured\":{\"path\":\"/private/window.png\"}}\n").await.unwrap();
    let _: () = decode(&client.line().await.unwrap(), "Handled").unwrap();
    assert_eq!(client.event().await.unwrap()["WindowClosed"]["id"], 2);
    assert_eq!(
        client.event().await.unwrap()["ScreenshotCaptured"]["path"],
        "/private/window.png"
    );
}

/// 【niri 回归】【传输上限】超限且未换行的来源必须结束读取，无参数、无返回值
#[tokio::test]
async fn oversized_reply_is_rejected_before_unbounded_allocation() {
    let (mut client, mut server) = pair();
    let sender = tokio::spawn(async move {
        let _ = server.write_all(&vec![b'x'; MAX_REPLY as usize + 1]).await;
    });
    assert!(
        client
            .line()
            .await
            .unwrap_err()
            .to_string()
            .contains("oversized")
    );
    drop(client);
    sender.await.unwrap();
}

/// 【niri 回归】【断连边界】半行 JSON 后断连不得视为完整响应，无参数、无返回值
#[tokio::test]
async fn incomplete_reply_is_rejected() {
    let (mut client, mut server) = pair();
    server.write_all(b"{\"Ok\":").await.unwrap();
    drop(server);
    assert!(
        client
            .line()
            .await
            .unwrap_err()
            .to_string()
            .contains("incomplete")
    );
}

/// 【niri 回归】【身份隔离】错误进程的套接字不得接收任何请求，无参数、无返回值
#[tokio::test]
async fn mismatched_compositor_is_rejected_before_request() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("ipc.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let (probe, _) = UnixStream::pair().unwrap();
    let mut peer = Peer::read(&probe).unwrap();
    peer.pid += 1;
    let result = Client::connect(&path, peer).await;
    assert!(
        result
            .err()
            .unwrap()
            .to_string()
            .contains("different compositor")
    );
    let (mut accepted, _) = listener.accept().await.unwrap();
    use tokio::io::AsyncReadExt;
    assert_eq!(accepted.read(&mut [0; 1]).await.unwrap(), 0);
}

/// 【niri 回归】【响应校验】保留服务端错误并拒绝错误响应类型，无参数、无返回值
#[test]
fn reply_errors_and_variants_remain_distinct() {
    assert!(
        decode::<Value>(b"{\"Err\":\"window disappeared\"}\n", "FocusedWindow")
            .unwrap_err()
            .to_string()
            .contains("window disappeared")
    );
    assert!(
        decode::<Value>(b"{\"Ok\":{\"Outputs\":{}}}\n", "FocusedWindow")
            .unwrap_err()
            .to_string()
            .contains("expected FocusedWindow")
    );
}

/// 【niri 回归】【异步取消】请求未收到回复时取消等待并关闭连接，服务端须立即看到断连，无参数、无返回值
#[tokio::test]
async fn stalled_request_is_cancellable_without_a_waiting_worker() {
    use tokio::io::AsyncReadExt;
    let (mut client, mut server) = pair();
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(20),
        client.query::<Value>(&json!("FocusedWindow"), "FocusedWindow"),
    )
    .await;
    assert!(result.is_err());
    drop(client);
    let mut bytes = Vec::new();
    server.read_to_end(&mut bytes).await.unwrap();
    assert_eq!(bytes, b"\"FocusedWindow\"\n");
}
