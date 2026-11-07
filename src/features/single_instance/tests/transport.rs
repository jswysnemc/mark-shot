//! 【单实例回归】【真实传输】同一私有路径只保留一个占用者，有限连接完成后释放

use super::super::{Entry, Owner, peer::Peer, request::MAX_PACKET};
use std::{
    io::{Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    time::{Duration, Instant},
};

/// 【单实例回归】【重复启动】真实文件锁与 Unix 请求返回已转交，原服务保持占用，退出后同路径可以重新启动，无参数、无返回值
#[tokio::test(flavor = "current_thread")]
async fn duplicate_entry_forwards_and_owner_releases_endpoint_before_unlocking() {
    let root = tempfile::tempdir().unwrap();
    let path = super::path(root.path(), 10);
    let (request, plan) = super::capture();
    let Entry::Owner(owner) = Owner::start_at(path.clone(), request.clone(), plan)
        .await
        .unwrap()
    else {
        panic!("expected owner");
    };
    owner.controller.active();
    let controller = owner.controller.clone();
    tokio::select! {
        result = Owner::start_at(path.clone(), request.clone(), plan) => assert!(matches!(result.unwrap(), Entry::Forwarded)),
        error = controller.serve() => panic!("service stopped: {error:?}"),
    }
    assert!(super::socket_exists(&path.socket));
    assert!(controller.poll_guard().deadline().is_none());
    drop(owner);
    assert!(!path.socket.exists());
    let Entry::Owner(next) = Owner::start_at(path.clone(), request, plan).await.unwrap() else {
        panic!("expected replacement owner");
    };
    drop(controller);
    assert!(super::socket_exists(&path.socket));
    drop(next);
    assert!(!path.socket.exists());
}

/// 【单实例回归】【失效恢复】仅恢复已经关闭且仍在私有路径的旧套接字，无参数、无返回值
#[tokio::test(flavor = "current_thread")]
async fn stale_socket_is_recovered_under_the_file_lock() {
    let root = tempfile::tempdir().unwrap();
    let path = super::path(root.path(), 11);
    drop(UnixListener::bind(&path.socket).unwrap());
    let (request, plan) = super::capture();
    let Entry::Owner(owner) = Owner::start_at(path.clone(), request, plan).await.unwrap() else {
        panic!("expected stale socket recovery");
    };
    assert!(super::socket_exists(&path.socket));
    drop(owner);
    assert!(!path.socket.exists());
}

/// 【单实例回归】【分段与期限】未补全的请求不调用业务，超过期限后释放连接，无参数、无返回值
#[tokio::test(flavor = "current_thread")]
async fn incomplete_request_expires_without_invoking_capture() {
    let (socket, mut client) = UnixStream::pair().unwrap();
    let now = Instant::now();
    let mut peer = Peer::new(socket, nix::unistd::Uid::effective().as_raw(), now).unwrap();
    client.write_all(b"{\"capture\":").unwrap();
    let mut calls = 0;
    let mut handler = |_| {
        calls += 1;
        Vec::new()
    };
    assert!(peer.progress(now, &mut handler).unwrap());
    assert!(
        !peer
            .progress(now + Duration::from_secs(2), &mut handler)
            .unwrap()
    );
    assert_eq!(calls, 0);
    drop(peer);
    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).unwrap();
    assert!(bytes.is_empty());
}

/// 【单实例回归】【正文上限】超过上限的输入收到明确拒绝并关闭，不调用截图业务，无参数、无返回值
#[tokio::test(flavor = "current_thread")]
async fn oversized_request_is_rejected_without_business_delivery() {
    let (socket, mut client) = UnixStream::pair().unwrap();
    client.write_all(&vec![b' '; MAX_PACKET + 1]).unwrap();
    let now = Instant::now();
    let mut peer = Peer::new(socket, nix::unistd::Uid::effective().as_raw(), now).unwrap();
    let mut calls = 0;
    assert!(
        !peer
            .progress(now, &mut |_| {
                calls += 1;
                Vec::new()
            })
            .unwrap()
    );
    assert_eq!(calls, 0);
    drop(peer);
    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).unwrap();
    let reply: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(reply["handled"], false);
}

/// 【单实例回归】【业务交付一次】分段请求在完整换行后只交付一次，回复结束后释放连接，无参数、无返回值
#[tokio::test(flavor = "current_thread")]
async fn complete_fragmented_request_is_delivered_once() {
    let (socket, mut client) = UnixStream::pair().unwrap();
    let now = Instant::now();
    let mut peer = Peer::new(socket, nix::unistd::Uid::effective().as_raw(), now).unwrap();
    let mut calls = 0;
    client.write_all(b"{\"capture\":").unwrap();
    assert!(
        peer.progress(now, &mut |_| {
            calls += 1;
            Vec::new()
        })
        .unwrap()
    );
    client.write_all(b"true}\n").unwrap();
    assert!(
        !peer
            .progress(now, &mut |request| {
                assert!(request.capture);
                calls += 1;
                b"{\"handled\":true}\n".to_vec()
            })
            .unwrap()
    );
    assert_eq!(calls, 1);
    drop(peer);
    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"{\"handled\":true}\n");
}
