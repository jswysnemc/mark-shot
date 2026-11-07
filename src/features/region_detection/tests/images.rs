//! 【区域识别回归】【实际图像】比较完整中间像素摘要和原版候选

use super::{helpers, selection};
use crate::features::region_detection::{
    Detector, analysis, edges,
    image::{Rgb, canny, components, morphology, resize},
};
use std::borrow::Cow;

/// 【区域识别回归】【图像读取】按 input 加载固定原图并应用格式，返回 RGBA 字节与整数尺寸
fn image(input: &serde_json::Value) -> (Vec<u8>, i32, i32) {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(input["image"].as_str().unwrap());
    let image = image::open(path).unwrap().into_rgba8();
    let (width, height) = image.dimensions();
    let mut pixels = image.into_raw();
    if input["premultiplied"].as_bool().unwrap_or(false) {
        crate::utils::premultiply(&mut pixels);
    }
    (pixels, width as i32, height as i32)
}

/// 【区域识别回归】【分析步骤】验证方向、Canny、形态和连通域的全部字节摘要，无参数、无返回值
#[test]
fn analysis_pixels_and_components_match_qt() {
    let (inputs, expected) = helpers::fixture();
    for input in inputs.iter().filter(|input| input["mode"] == "image") {
        let name = input["name"].as_str().unwrap();
        let (pixels, width, height) = image(input);
        let source = Rgb::new(
            width as usize,
            height as usize,
            &pixels,
            input["premultiplied"].as_bool().unwrap(),
        )
        .unwrap();
        let size = analysis::size(source.width, source.height);
        let image = resize::downscale(&source, size.0, size.1);
        let steps = &expected[name]["steps"];
        assert_eq!(
            serde_json::json!([size.0, size.1]),
            steps["size"],
            "{name}: analysis size"
        );
        assert_eq!(
            helpers::hash(&image.pixels),
            steps["rgba"],
            "{name}: resized RGBA"
        );
        let gray = image.grayscale();
        let field = edges::build(&image);
        let high = (f32::from(field.threshold) * 3.0).clamp(15.0, 54.0);
        let canny = canny::detect(&gray, high * 0.4, high);
        let closed = morphology::close(&field.combined);
        let dilated = morphology::dilate(&field.combined);
        for (label, bytes) in [
            ("gray", &gray.pixels),
            ("canny", &canny.pixels),
            ("vertical", &field.vertical.pixels),
            ("horizontal", &field.horizontal.pixels),
            ("combined", &field.combined.pixels),
            ("closed", &closed.pixels),
            ("dilated", &dilated.pixels),
        ] {
            assert_eq!(helpers::hash(bytes), steps[label], "{name}: {label}");
        }
        assert_eq!(
            u64::from(field.threshold),
            steps["threshold"].as_u64().unwrap(),
            "{name}: threshold"
        );
        for (label, eight) in [("components_four", false), ("components_eight", true)] {
            let actual: Vec<_> = components::connected(&closed, eight).into_iter().map(|item| serde_json::json!({"rect": helpers::encoded(item.rect), "pixels": item.pixels})).collect();
            assert_eq!(
                serde_json::Value::Array(actual),
                steps[label],
                "{name}: {label}"
            );
        }
    }
}

/// 【区域识别回归】【图像候选】验证完整图像检测、精修、来源和悬停动作，无参数、无返回值
#[test]
fn detected_regions_and_hierarchy_match_qt() {
    let (inputs, expected) = helpers::fixture();
    for input in inputs.iter().filter(|input| input["mode"] == "image") {
        let name = input["name"].as_str().unwrap();
        let (pixels, width, height) = image(input);
        let mut detector = Detector::from_rgba(
            width,
            height,
            &pixels,
            input["premultiplied"].as_bool().unwrap(),
        )
        .unwrap();
        let windows: Vec<_> = input["window_rects"]
            .as_array()
            .unwrap()
            .iter()
            .map(helpers::rect)
            .collect();
        detector.add_windows(&windows);
        helpers::candidates(detector.candidates(), &expected[name]["candidates"], name);
        selection::actions(&detector, input, &expected[name]);
    }
}

/// 【区域识别回归】【原图借用】不透明预乘截图不得复制原图，透明格式独立归一化，非法长度明确拒绝，无参数、无返回值
#[test]
fn opaque_rgba_is_borrowed_and_dimensions_are_checked() {
    let opaque = [40, 60, 80, 255];
    assert!(matches!(
        Rgb::new(1, 1, &opaque, true).unwrap().pixels,
        Cow::Borrowed(_)
    ));
    let transparent = [20, 30, 40, 128];
    assert!(matches!(
        Rgb::new(1, 1, &transparent, true).unwrap().pixels,
        Cow::Owned(_)
    ));
    assert!(Detector::from_rgba(2, 1, &opaque, true).is_err());
    assert!(Detector::from_rgba(-1, 1, &[], true).is_err());
}
