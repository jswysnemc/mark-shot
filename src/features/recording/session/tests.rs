//! 【进程内录制回归】【真实桌面】显式私有会话录制并检查暂停、结果与无外部采集进程

use super::*;

/// 【进程内录制回归】【完整控制】无参数，在指定私有会话录制，产物供独立像素与音视频验证
#[test]
#[ignore = "requires MARK_SHOT_PRIVATE_RECORDING_TEST and MARK_SHOT_MEDIA_LIBRARY"]
fn private_native_recording_pause_resume() {
    assert_eq!(
        std::env::var("MARK_SHOT_PRIVATE_RECORDING_TEST").as_deref(),
        Ok("1")
    );
    let directory = std::path::PathBuf::from(std::env::var_os("MARK_SHOT_MEDIA_EVIDENCE").unwrap());
    std::fs::create_dir_all(&directory).unwrap();
    let config = Recording {
        encoder: std::env::var("MARK_SHOT_TEST_ENCODER").unwrap_or_else(|_| "software".into()),
        countdown: 0,
        audio: std::env::var("MARK_SHOT_TEST_AUDIO").unwrap_or_else(|_| "off".into()),
        cursor: false,
        fps: 30,
        ..Default::default()
    };
    let target = Target {
        display: std::env::var("MARK_SHOT_TEST_OUTPUT").unwrap_or_default(),
        crop: Some([20, 40, 320, 240]),
        format: None,
    };
    let status = std::sync::Arc::new(std::sync::Mutex::new(super::super::Status::default()));
    let (send, receive) = std::sync::mpsc::channel();
    let state = status.clone();
    let control = std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(12);
        while state.lock().unwrap().phase != "recording" {
            assert!(
                Instant::now() < until,
                "recording did not start: {}",
                state.lock().unwrap().detail
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(Duration::from_secs(2));
        send.send(Command::Pause).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        let paused = state.lock().unwrap().elapsed_ms;
        assert_eq!(state.lock().unwrap().phase, "paused");
        std::thread::sleep(Duration::from_secs(1));
        assert_eq!(state.lock().unwrap().elapsed_ms, paused);
        if std::env::var("MARK_SHOT_TEST_STOP_PAUSED").as_deref() != Ok("1") {
            send.send(Command::Resume).unwrap();
            std::thread::sleep(Duration::from_secs(2));
        }
        send.send(Command::Stop).unwrap();
    });
    let result = run(
        &config,
        &target,
        &directory.join("desktop.mp4"),
        &receive,
        &status,
    );
    assert!(result.is_ok(), "{result:?}");
    control.join().unwrap();
    let state = status.lock().unwrap();
    let duration = if std::env::var("MARK_SHOT_TEST_STOP_PAUSED").as_deref() == Ok("1") {
        1800..2600
    } else {
        3500..4800
    };
    assert!(duration.contains(&state.elapsed_ms), "{}", state.elapsed_ms);
    assert!(state.detail.contains("in-process"));
    if let Ok(expected) = std::env::var("MARK_SHOT_TEST_CAPTURE_PATH") {
        assert!(state.detail.contains(&expected), "{}", state.detail);
    }
    std::fs::write(
        directory.join("status.json"),
        serde_json::to_vec_pretty(&*state).unwrap(),
    )
    .unwrap();
}

/// 【进程内录制回归】【区域保护】无参数，合法非零区域保持原值，奇数尺寸和加法溢出拒绝
#[test]
fn native_crop_keeps_origin_and_rejects_overflow() {
    assert_eq!(
        crop(Some([20, 40, 320, 240]), (1280, 720)).unwrap(),
        [20, 40, 320, 240]
    );
    assert_eq!(crop(None, (1281, 721)).unwrap(), [0, 0, 1280, 720]);
    assert!(crop(Some([u32::MAX, 0, 320, 240]), (1280, 720)).is_err());
    assert!(crop(Some([0, 0, 319, 240]), (1280, 720)).is_err());
}
