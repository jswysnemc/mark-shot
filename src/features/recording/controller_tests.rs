//! 【录制测试】【取消启动】在不创建采集设备的条件下验证工作目录和线程收尾

use super::*;
use crate::config::recording::Recording;

/// 【录制测试】【倒计时取消】无参数，验证停止唤醒倒计时、不发布成品且清理工作目录
#[test]
fn stopping_countdown_leaves_no_recording_files() {
    let directory = tempfile::tempdir().unwrap();
    let controller = Controller::start(
        Recording {
            countdown: 10,
            directory: directory.path().to_string_lossy().into(),
            ..Default::default()
        },
        Target::default(),
    )
    .unwrap();
    controller.send(Command::Stop).unwrap();
    let until = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !controller.finished() && std::time::Instant::now() < until {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(controller.finished());
    assert_eq!(controller.snapshot().phase, "cancelled");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

/// 【录制测试】【非法配置】无参数，验证无效帧率在创建文件或线程之前返回错误
#[test]
fn invalid_configuration_does_not_reserve_output() {
    let directory = tempfile::tempdir().unwrap();
    assert!(
        Controller::start(
            Recording {
                fps: 0,
                directory: directory.path().to_string_lossy().into(),
                ..Default::default()
            },
            Target::default(),
        )
        .is_err()
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}
