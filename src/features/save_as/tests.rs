//! 【截图另存为测试】【文件事实】验证冻结字节、真实完成结果与临时线程回收

use super::{
    Snapshot,
    jobs::{Jobs, Response, Task},
};
use crate::backend::window::WindowWake;
use std::{
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

/// 【截图另存为测试】【图像数据】返回含透明像素的 PNG 原始字节，无外部文件或模型依赖
fn png() -> Vec<u8> {
    let mut pixmap = tiny_skia::Pixmap::new(17, 9).unwrap();
    pixmap.fill(tiny_skia::Color::from_rgba8(200, 100, 50, 128));
    crate::utils::encode_png_with_alpha(pixmap)
}

/// 【截图另存为测试】【结果等待】在 timeout 内接收 jobs 的一个完成响应；返回响应，失败时结束测试
fn response(jobs: &mut Jobs) -> Response {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(response) = jobs.take() {
            return response;
        }
        assert!(Instant::now() < deadline, "file worker did not complete");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// 【截图另存为测试】【共享原件】克隆快照只共享 PNG 字节，不重编码、缩放或丢失透明度
#[test]
fn snapshot_clones_share_the_exact_original_png() {
    let data = png();
    let snapshot = Snapshot::new(data.clone(), 1.25, "search".into()).unwrap();
    let copy = snapshot.clone();
    assert!(Arc::ptr_eq(&snapshot.png, &copy.png));
    assert_eq!(*copy.png, data);
    assert_eq!(copy.scale, 1.25);
    assert_eq!(copy.search, "search");
    assert!(Snapshot::new(b"broken".to_vec(), 1.0, String::new()).is_err());
    assert!(Snapshot::new(png(), f32::NAN, String::new()).is_err());
}

/// 【截图另存为测试】【关闭完成】窗口没有读取任务响应也必须识别已完成保存；关闭回收后原 PNG 完整存在
#[test]
fn closing_the_window_does_not_misreport_a_completed_save_as_cancel() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("image.png");
    let original = Arc::new(png());
    let completion = Arc::new(OnceLock::new());
    let mut jobs = Jobs::new(WindowWake::detached().unwrap(), completion.clone());
    jobs.submit(Task::Save {
        path: path.clone(),
        expected: None,
        png: original.clone(),
    });
    drop(jobs);
    assert_eq!(completion.get(), Some(&Ok(path.clone())));
    assert_eq!(std::fs::read(path).unwrap(), *original);
}

/// 【截图另存为测试】【失败事实】写入不存在目录必须返回失败，不建立虚假完成结果或保存图像
#[test]
fn failed_saves_are_reported_and_do_not_create_an_image() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("missing/image.png");
    let completion = Arc::new(OnceLock::new());
    let mut jobs = Jobs::new(WindowWake::detached().unwrap(), completion.clone());
    jobs.submit(Task::Save {
        path: path.clone(),
        expected: None,
        png: Arc::new(png()),
    });
    assert!(matches!(response(&mut jobs), Response::Saved(Err(_))));
    assert!(completion.get().unwrap().is_err());
    assert!(!path.exists());
}

/// 【截图另存为测试】【目录与取消】只浏览默认目录不产生完成结果，任务结束后不保留工作线程
#[test]
fn directory_browsing_does_not_complete_or_save_a_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("new-directory");
    let completion = Arc::new(OnceLock::new());
    let mut jobs = Jobs::new(WindowWake::detached().unwrap(), completion.clone());
    jobs.submit(Task::List {
        path: path.clone(),
        hidden: false,
        create: true,
    });
    match response(&mut jobs) {
        Response::Listed { requested, listing } => {
            assert_eq!(requested, path);
            assert!(listing.unwrap().items.is_empty());
        }
        _ => panic!("directory browsing returned a save result"),
    }
    assert!(completion.get().is_none());
    assert_eq!(std::fs::read_dir(path).unwrap().count(), 0);
    drop(jobs);
}
