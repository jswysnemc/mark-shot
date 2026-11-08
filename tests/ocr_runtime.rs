#![cfg(all(target_os = "linux", feature = "ocr-dynamic"))]

use mark_shot::ocr::{
    self,
    models::{ModelFiles, ModelStatus},
    runtime_library,
};
use std::{
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

/// 【识别运行库】【隔离进程】以 name 重新执行当前测试，设置独立数据目录；返回是否已在子进程
fn isolated(name: &str, ignored: bool) -> bool {
    if std::env::var("MARK_SHOT_RUNTIME_CHILD").as_deref() == Ok(name) {
        return true;
    }
    let root = tempfile::tempdir().unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args(["--exact", name, "--nocapture"]);
    if ignored {
        command.arg("--ignored");
    }
    let output = command
        .env("MARK_SHOT_RUNTIME_CHILD", name)
        .env("XDG_DATA_HOME", root.path().join("data"))
        .env("XDG_STATE_HOME", root.path().join("state"))
        .env("XDG_CONFIG_HOME", root.path().join("config"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    print!("{}", String::from_utf8_lossy(&output.stdout));
    false
}

/// 【识别运行库】【首次运行回归】无参数，验证普通初始化与加载失败不创建运行库文件或安装任务
#[test]
fn first_use_never_starts_a_download() {
    if !isolated("first_use_never_starts_a_download", false) {
        return;
    }
    let models = ocr::models::OcrModels::load();
    assert!(!models.needs_poll());
    assert_eq!(models.library.status, ModelStatus::Missing);
    assert!(ocr::engine_choice::source(&models).is_none());
    assert!(
        runtime_library::load()
            .unwrap_err()
            .contains("Install or repair")
    );
    assert!(!runtime_library::directory().unwrap().exists());
}

/// 【识别运行库】【实际识别回归】使用环境变量指定的官方归档和模型，显式安装后识别固定图片；不下载测试资源
#[test]
#[ignore = "requires MARK_SHOT_TEST_ORT_ARCHIVE and MARK_SHOT_TEST_OCR_MODELS"]
fn explicit_install_runs_real_ocr_offline() {
    if !isolated("explicit_install_runs_real_ocr_offline", true) {
        return;
    }
    let archive =
        PathBuf::from(std::env::var_os("MARK_SHOT_TEST_ORT_ARCHIVE").expect("official archive"));
    let models =
        PathBuf::from(std::env::var_os("MARK_SHOT_TEST_OCR_MODELS").expect("verified OCR models"));
    let root = runtime_library::directory().unwrap();
    std::fs::create_dir_all(&root).unwrap();
    std::fs::copy(
        archive,
        root.join(runtime_library::manifest::current().unwrap().archive.file),
    )
    .unwrap();
    let mut installer = runtime_library::Installer::default();
    assert_eq!(installer.status, ModelStatus::Missing);
    assert!(!installer.needs_poll());
    installer.start();
    let deadline = Instant::now() + Duration::from_secs(30);
    while installer.needs_poll() {
        installer.poll();
        assert!(Instant::now() < deadline, "installation timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        installer.status,
        ModelStatus::Installed,
        "{:?}",
        installer.error
    );
    let files = ModelFiles {
        detector: models.join("pp-ocrv5_mobile_det.onnx"),
        recognizer: models.join("pp-ocrv5_mobile_rec.onnx"),
        dict: models.join("ppocrv5_dict.txt"),
    };
    let backend = ocr::default_backend(&files).unwrap();
    let image = image::open(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ocr.png"
    ))
    .unwrap()
    .to_rgb8();
    let output = backend
        .recognize(
            ocr::OcrImage {
                width: image.width(),
                height: image.height(),
                rgb: image.into_raw(),
                origin: (0.0, 0.0),
            },
            &|| false,
        )
        .unwrap();
    assert_eq!(
        output
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        [
            "Wayland Native Screenshot",
            "Mark Shot OCR 2026",
            "Local text recognition"
        ]
    );
    println!(
        "Recognized {} lines: {:?}",
        output.lines.len(),
        output
            .lines
            .iter()
            .map(|line| &line.text)
            .collect::<Vec<_>>()
    );
}
