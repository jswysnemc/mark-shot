use super::*;

#[test]
fn a_measurement_survives_a_save_and_a_reload() {
    let dir = testing::temp_dir("calib-roundtrip");
    let path = dir.join("ocr-device");
    let record = measured(Verdict::Gpu, Some(1650), Some(680));
    calibrate::store(&path, &record, "card-a");
    assert_eq!(calibrate::stored(&path, "card-a"), Some(record));
}

#[test]
fn a_measurement_from_other_hardware_is_ignored() {
    let dir = testing::temp_dir("calib-foreign");
    let path = dir.join("ocr-device");
    calibrate::store(
        &path,
        &measured(Verdict::Gpu, Some(1650), Some(680)),
        "card-a",
    );
    assert_eq!(calibrate::stored(&path, "card-b"), None);
    assert_eq!(calibrate::stored(&dir.join("nothing-here"), "card-a"), None);
}

#[test]
fn a_damaged_measurement_is_ignored() {
    for text in [
        "",
        "garbage",
        "verdict = gpu\n",
        "format = 1\nkey = k\nverdict = maybe\n",
        "format = 99\nkey = k\nverdict = gpu\n",
        "format = 1\nkey = other\nverdict = gpu\n",
    ] {
        assert_eq!(calibrate::parse(text, "k"), None, "accepted {text:?}");
    }
}

#[test]
fn an_engine_that_could_not_be_measured_is_kept_as_failed() {
    let dir = testing::temp_dir("calib-failed");
    let path = dir.join("ocr-device");
    let record = measured(Verdict::Cpu, Some(1500), None);
    calibrate::store(&path, &record, "card-a");
    assert!(
        fs::read_to_string(&path)
            .unwrap()
            .contains("gpu_ms = failed")
    );
    assert_eq!(calibrate::stored(&path, "card-a"), Some(record));
}

#[test]
fn the_gpu_has_to_win_by_a_margin() {
    assert_eq!(calibrate::decide(Some(1650), Some(680)), Verdict::Gpu);
    assert_eq!(calibrate::decide(Some(1000), Some(800)), Verdict::Gpu);
    assert_eq!(calibrate::decide(Some(1000), Some(801)), Verdict::Cpu);
    assert_eq!(calibrate::decide(Some(2000), Some(40_000)), Verdict::Cpu);
    assert_eq!(calibrate::decide(Some(1000), None), Verdict::Cpu);
    assert_eq!(calibrate::decide(None, Some(500)), Verdict::Gpu);
    assert_eq!(calibrate::decide(None, None), Verdict::Cpu);
}

#[test]
fn the_probe_image_is_the_same_every_time() {
    let first = calibrate::probe_image();
    let again = calibrate::probe_image();
    assert_eq!(first.rgb, again.rgb);
    assert_eq!(
        first.rgb.len(),
        (first.width as usize) * (first.height as usize) * 3
    );
    assert!(
        first.rgb.iter().any(|&value| value < 0x40),
        "no dark bars to detect"
    );
    assert!(
        first.rgb.iter().any(|&value| value > 0xC0),
        "no light background"
    );
}

#[test]
fn the_machine_key_does_not_drift() {
    let key = calibrate::machine_key();
    assert_eq!(key, calibrate::machine_key());
    assert!(key.starts_with(&format!("v{}", calibrate::FORMAT)), "{key}");
    assert!(key.contains(env!("CARGO_PKG_VERSION")), "{key}");
}

#[test]
fn a_summary_names_the_winner_and_the_numbers() {
    let gpu = measured(Verdict::Gpu, Some(1650), Some(680)).summary();
    assert!(gpu.contains("GPU is 2.4x faster"), "{gpu}");
    assert!(
        gpu.contains("CPU 1650 ms") && gpu.contains("GPU 680 ms"),
        "{gpu}"
    );

    let cpu = measured(Verdict::Cpu, Some(2000), Some(40_000)).summary();
    assert!(cpu.contains("CPU is 20.0x faster"), "{cpu}");

    let broken = measured(Verdict::Cpu, Some(1500), None).summary();
    assert!(broken.contains("GPU failed"), "{broken}");
}

#[test]
#[ignore]
fn auto_lands_on_the_device_this_machine_measured() {
    let models = crate::ocr::models::OcrModels::load();
    let files = models
        .active()
        .and_then(|idx| models.files(idx))
        .expect("install an OCR model first");
    let record = super::super::measurement().expect("run --ocr-daemon calibrate first");

    let (dir, paths) = dir_and_paths("auto-real");
    let factory = super::super::engine_factory(crate::ocr::settings::Device::Auto, paths.clone());
    let server = start(config(&dir, factory));
    load(&paths, &files);
    assert!(wait_until(Duration::from_secs(300), || matches!(
        state(&paths),
        Some(EngineState::Ready { .. } | EngineState::NoGpu)
    )));

    match (record.verdict, state(&paths)) {
        (Verdict::Gpu, Some(EngineState::Ready { device, .. })) => {
            assert_eq!(device, "webgpu");
            shutdown(&paths);
        }
        (Verdict::Cpu, Some(EngineState::NoGpu)) => {}
        (verdict, reached) => panic!("measured {verdict:?} but the daemon reached {reached:?}"),
    }
    finish(server);
}
