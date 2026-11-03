use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::thread::{self, JoinHandle};

use mark_shot::ocr::download;
use mark_shot::ocr::model_integrity;
use mark_shot::ocr::models::Asset;

const ASSET: Asset = Asset {
    file: "model.onnx",
    size: 3,
    sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
};

/// 【模型下载测试】【本地服务】返回固定 HTTP 数据并记录请求；返回下载地址和服务线程
fn server(status: &str, body: &'static [u8], content_range: &str) -> (String, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let status = status.to_owned();
    let content_range = content_range.to_owned();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        let mut request = Vec::new();
        let mut byte = [0];
        while !request.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{content_range}Connection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(body).unwrap();
        String::from_utf8(request).unwrap()
    });
    (format!("http://{address}/model.onnx"), handle)
}

/// 【模型下载测试】【下载完成】验证只有长度和 SHA-256 一致的资源会成为正式模型文件
#[test]
fn verified_download_is_committed_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let (url, server) = server("200 OK", b"abc", "");
    let mut progress = Vec::new();
    download::fetch(
        &download::agent(),
        &url,
        &ASSET,
        directory.path(),
        &AtomicBool::new(false),
        &AtomicBool::new(false),
        |bytes| progress.push(bytes),
    )
    .unwrap();
    server.join().unwrap();
    assert_eq!(
        std::fs::read(directory.path().join(ASSET.file)).unwrap(),
        b"abc"
    );
    assert!(!directory.path().join("model.onnx.part").exists());
    assert_eq!(progress.last(), Some(&3));
    model_integrity::verify_asset(&directory.path().join(ASSET.file), &ASSET).unwrap();
}

/// 【模型下载测试】【断点续传】验证已下载前缀通过 Range 继续获取并校验完整文件
#[test]
fn partial_download_resumes_at_exact_offset() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("model.onnx.part"), b"a").unwrap();
    let (url, server) = server(
        "206 Partial Content",
        b"bc",
        "Content-Range: bytes 1-2/3\r\n",
    );
    download::fetch(
        &download::agent(),
        &url,
        &ASSET,
        directory.path(),
        &AtomicBool::new(false),
        &AtomicBool::new(false),
        |_| {},
    )
    .unwrap();
    let request = server.join().unwrap().to_ascii_lowercase();
    assert!(request.contains("range: bytes=1-"));
    assert_eq!(
        std::fs::read(directory.path().join(ASSET.file)).unwrap(),
        b"abc"
    );
}

/// 【模型下载测试】【损坏资源】验证相同长度但错误校验和的模型不会安装，也不会保留错误前缀
#[test]
fn checksum_failure_never_installs_the_model() {
    let directory = tempfile::tempdir().unwrap();
    let (url, server) = server("200 OK", b"abd", "");
    let result = download::fetch(
        &download::agent(),
        &url,
        &ASSET,
        directory.path(),
        &AtomicBool::new(false),
        &AtomicBool::new(false),
        |_| {},
    );
    server.join().unwrap();
    assert!(result.unwrap_err().contains("checksum"));
    assert!(!directory.path().join(ASSET.file).exists());
    assert!(!directory.path().join("model.onnx.part").exists());
}

/// 【模型下载测试】【取消请求】验证提前取消不创建网络请求或模型文件
#[test]
fn cancelled_download_does_not_access_network() {
    let directory = tempfile::tempdir().unwrap();
    let result = download::fetch(
        &download::agent(),
        "http://127.0.0.1:1/not-used",
        &ASSET,
        directory.path(),
        &AtomicBool::new(true),
        &AtomicBool::new(false),
        |_| {},
    );
    assert_eq!(result, Err("cancelled".into()));
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

/// 【模型下载测试】【缓存完整性】验证识别前检查能发现与正常模型长度相同的损坏缓存
#[test]
fn engine_validation_detects_same_size_corruption() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(ASSET.file);
    std::fs::write(&path, b"abd").unwrap();
    let error = model_integrity::verify_asset(&path, &ASSET).unwrap_err();
    assert!(error.reason.contains("SHA-256"));
    assert_eq!(error.path, path);
}
