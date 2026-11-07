//! 【Sway 窗口识别回归】【协议边界】有界传输、工作区隔离与窗口层级

use super::*;
use serde_json::json;

/// 【Sway 窗口识别回归】【窗口数据】按 id 与 x 返回具有真实窗口身份的树叶节点
fn leaf(id: u64, x: i32) -> Value {
    json!({"id":id,"type":"con","app_id":"fixture","rect":{"x":x,"y":20,"width":100,"height":80},"nodes":[],"floating_nodes":[]})
}

/// 【Sway 窗口识别回归】【活动工作区】其他工作区不可命中，浮动窗口高于平铺窗口，无参数、无返回值
#[test]
fn current_workspace_and_floating_z_order_are_preserved() {
    let tree = json!({"nodes":[{"type":"output","name":"DP-1","current_workspace":"one","nodes":[
        {"type":"workspace","name":"one","nodes":[leaf(1,10),leaf(2,30)],"focus":[2,1],"floating_nodes":[leaf(3,50)]},
        {"type":"workspace","name":"two","nodes":[leaf(4,500)]}
    ]}]});
    let actual = windows(&tree);
    assert_eq!(actual.len(), 3);
    assert_eq!(
        actual
            .iter()
            .map(|window| window.rect.left())
            .collect::<Vec<_>>(),
        [10.0, 30.0, 50.0]
    );
    assert!(actual[1].z_order > actual[0].z_order);
    assert!(actual[2].z_order > actual[1].z_order);
}

/// 【Sway 窗口识别回归】【全屏覆盖】全局全屏窗口阻止隐藏桌面参与选择，无参数、无返回值
#[test]
fn global_fullscreen_excludes_other_outputs() {
    let mut window = leaf(1, 0);
    window["fullscreen_mode"] = json!(2);
    let tree = json!({"nodes":[window,leaf(2,500)]});
    let actual = windows(&tree);
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0].rect.left(), 0.0);
}

/// 【Sway 窗口识别回归】【超限帧头】服务端声明超限时在读取正文前拒绝，无参数、无返回值
#[tokio::test]
async fn oversized_reply_is_rejected_without_reading_payload() {
    let (mut client, mut server) = UnixStream::pair().unwrap();
    let sender = tokio::spawn(async move {
        let mut request = [0u8; 14];
        server.read_exact(&mut request).await.unwrap();
        assert_eq!(&request[..6], MAGIC);
        assert_eq!(u32::from_le_bytes(request[10..].try_into().unwrap()), TREE);
        request[6..10].copy_from_slice(&(MAX_REPLY + 1).to_le_bytes());
        server.write_all(&request).await.unwrap();
    });
    assert!(
        tree(&mut client)
            .await
            .unwrap_err()
            .to_string()
            .contains("oversized")
    );
    sender.await.unwrap();
}

/// 【Sway 窗口识别回归】【部分帧】回复正文断连不得当作成功快照，无参数、无返回值
#[tokio::test]
async fn partial_payload_is_rejected() {
    let (mut client, mut server) = UnixStream::pair().unwrap();
    let sender = tokio::spawn(async move {
        let mut request = [0u8; 14];
        server.read_exact(&mut request).await.unwrap();
        request[6..10].copy_from_slice(&20u32.to_le_bytes());
        server.write_all(&request).await.unwrap();
        server.write_all(b"{\"nodes\":").await.unwrap();
    });
    assert!(tree(&mut client).await.is_err());
    sender.await.unwrap();
}
