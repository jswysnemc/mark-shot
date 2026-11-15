//! 【截图窗口测试】【租约恢复】验证真实文件锁、监听通知、隐藏确认和异常恢复

use super::*;
use std::{sync::mpsc, time::Duration};

/// 【截图窗口测试】【确认顺序】无参数，捕获必须等待窗口确认，守卫释放后监听器立即恢复可见请求
#[test]
fn capture_waits_for_acknowledgement_and_drop_restores_windows() {
    let directory = tempfile::tempdir().unwrap();
    let (wake, events) = mpsc::channel();
    let mut watch = Watch::in_directory(directory.path(), move || {
        let _ = wake.send(());
    })
    .unwrap();
    let path = directory.path().to_owned();
    let (finished, result) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let guard = Guard::in_directory(&path, Duration::from_secs(2)).unwrap();
        finished.send(guard).unwrap();
    });
    while !watch.requested().unwrap() {
        events.recv_timeout(Duration::from_secs(2)).unwrap();
    }
    assert!(result.try_recv().is_err());
    watch.applied().unwrap();
    let guard = result.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(watch.requested().unwrap());
    drop(guard);
    assert!(!watch.requested().unwrap());
    watch.applied().unwrap();
    assert!(watch.deadline().is_none());
    worker.join().unwrap();
}

/// 【截图窗口测试】【超时回滚】无参数，窗口不确认时返回错误并撤回隐藏请求，不遗留锁定租约
#[test]
fn unresponsive_window_aborts_capture_and_releases_lease() {
    let directory = tempfile::tempdir().unwrap();
    let mut watch = Watch::in_directory(directory.path(), || {}).unwrap();
    assert!(Guard::in_directory(directory.path(), Duration::from_millis(30)).is_err());
    assert!(!watch.requested().unwrap());
    assert!(records(directory.path(), "lease-").unwrap().is_empty());
}

/// 【截图窗口测试】【进程异常】无参数，模拟进程退出后遗留文件，窗口只依据活跃文件锁决定隐藏
#[test]
fn unlocked_crash_leftover_does_not_keep_windows_hidden() {
    let directory = tempfile::tempdir().unwrap();
    let mut watch = Watch::in_directory(directory.path(), || {}).unwrap();
    let lease = record(directory.path(), "lease-").unwrap();
    assert!(watch.requested().unwrap());
    watch.applied().unwrap();
    let (file, path) = lease.keep().unwrap();
    drop(file);
    assert!(path.exists());
    assert!(!watch.requested().unwrap());
}

/// 【贴图编辑测试】【定向隔离】无参数，验证目标窗口隐藏并恢复，其他进程租约及窗口不会误隐藏或阻塞交接
#[test]
fn scoped_edit_lease_hides_only_the_origin_and_restores_it_on_drop() {
    use std::io::Write;
    let directory = tempfile::tempdir().unwrap();
    let mut watch = Watch::in_directory(directory.path(), || {}).unwrap();
    let other_pid = std::process::id().saturating_add(1);
    let other_window = record(directory.path(), &format!("window-{other_pid}-")).unwrap();
    let mut other_lease = record(directory.path(), &format!("lease-process-{other_pid}-")).unwrap();
    assert!(!watch.requested().unwrap());
    write!(other_lease.as_file_mut(), "process:{other_pid}").unwrap();
    assert!(!watch.requested().unwrap());
    drop(other_lease);
    let path = directory.path().to_owned();
    let worker = std::thread::spawn(move || {
        Guard::scoped(
            &path,
            Duration::from_secs(2),
            &format!("window-{}-", std::process::id()),
            &format!("process:{}", std::process::id()),
        )
        .unwrap()
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    while !watch.requested().unwrap() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    watch.applied().unwrap();
    let guard = worker.join().unwrap();
    assert!(watch.requested().unwrap());
    drop(guard);
    assert!(!watch.requested().unwrap());
    drop(other_window);
}
