//! 【图像导出测试】【原版像素】比较实际 Qt 导出的完整 RGBA，覆盖开关、参数、透明度、小图和分数 DPR

use mark_shot::{config::export::ImageFrame, features::export_image};
use serde::Deserialize;
use std::{borrow::Cow, path::PathBuf};

#[derive(Deserialize)]
struct Case {
    name: String,
    frame: serde_json::Value,
    size: [u32; 2],
    dpr: f64,
}

/// 【图像导出测试】【固定目录】返回当前仓库中原版生成的冻结样本目录，无参数
fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/qt-export-image")
}

/// 【图像导出测试】【完整对照】验证所有样本的尺寸和完整直通 RGBA 与原版 PNG 一致，无参数、无返回值
#[test]
fn exported_png_matches_actual_qt_pixels_in_every_reference_case() {
    let directory = directory();
    let cases: Vec<Case> =
        serde_json::from_slice(&std::fs::read(directory.join("cases.json")).unwrap()).unwrap();
    assert_eq!(cases.len(), 36);
    for case in cases {
        let png = std::fs::read(directory.join(format!("{}-source.png", case.name))).unwrap();
        let expected =
            std::fs::read(directory.join(format!("{}-expected.png", case.name))).unwrap();
        let frame = ImageFrame::from_value(&case.frame);
        let actual = export_image::prepare(&png, frame).unwrap();
        assert_eq!(matches!(actual, Cow::Borrowed(_)), !frame.enabled);
        let actual = image::load_from_memory(&actual).unwrap().into_rgba8();
        let expected = image::load_from_memory(&expected).unwrap().into_rgba8();
        assert_eq!(
            actual.dimensions(),
            (case.size[0], case.size[1]),
            "{}",
            case.name
        );
        if actual.as_raw() != expected.as_raw() {
            let differences: Vec<_> = actual
                .enumerate_pixels()
                .filter_map(|(x, y, pixel)| {
                    let reference = expected.get_pixel(x, y);
                    (pixel != reference).then_some((x, y, pixel.0, reference.0))
                })
                .collect();
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target/export-image-pixel-failures");
            std::fs::create_dir_all(&root).unwrap();
            actual
                .save(root.join(format!("{}-actual.png", case.name)))
                .unwrap();
            panic!(
                "{} DPR={} changed={} first={:?}",
                case.name,
                case.dpr,
                differences.len(),
                differences.first()
            );
        }
    }
}

/// 【图像导出测试】【零开销关闭】关闭外框不解码输入、不复制缓冲、不改变 PNG 文字块或元数据，无参数、无返回值
#[test]
fn disabled_frame_borrows_the_exact_original_bytes_without_decoding() {
    let png = b"existing PNG data and metadata";
    let output = export_image::prepare(png, ImageFrame::default()).unwrap();
    assert!(matches!(output, Cow::Borrowed(_)));
    assert_eq!(output.as_ptr(), png.as_ptr());
    assert_eq!(output.as_ref(), png);
    let output = export_image::for_delivery(png, false).unwrap();
    assert!(matches!(output, Cow::Borrowed(_)));
    assert_eq!(output.as_ptr(), png.as_ptr());
}

/// 【图像导出测试】【错误边界】开启外框时拒绝损坏图像、尺寸溢出和非有限参数，无参数、无返回值
#[test]
fn enabled_frame_rejects_corrupt_input_and_invalid_layout() {
    let frame = ImageFrame {
        enabled: true,
        ..ImageFrame::default()
    };
    assert!(export_image::prepare(b"invalid", frame).is_err());
    let png = std::fs::read(directory().join("case-0-source.png")).unwrap();
    for frame in [
        ImageFrame {
            padding: i32::MAX,
            ..frame
        },
        ImageFrame {
            corner_radius: f64::NAN,
            ..frame
        },
        ImageFrame {
            shadow_opacity: f64::INFINITY,
            ..frame
        },
        ImageFrame {
            shadow_radius: i32::MAX,
            ..frame
        },
    ] {
        assert!(export_image::prepare(&png, frame).is_err());
    }
    assert!(export_image::prepare(&png[..png.len() / 2], frame).is_err());
}

/// 【图像导出测试】【源图保持】合成结果不改变输入 PNG，连续导出从同一原图产生相同结果，无参数、无返回值
#[test]
fn preparing_multiple_exports_does_not_accumulate_frames_or_mutate_the_source() {
    let png = std::fs::read(directory().join("case-17-source.png")).unwrap();
    let original = png.clone();
    let frame = ImageFrame {
        enabled: true,
        ..ImageFrame::default()
    };
    let first = export_image::prepare(&png, frame).unwrap();
    let second = export_image::prepare(&png, frame).unwrap();
    assert_eq!(first, second);
    assert_eq!(png, original);
}
