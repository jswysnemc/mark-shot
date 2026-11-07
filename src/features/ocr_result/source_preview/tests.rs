//! 【原图测试】【实际 Qt 回放】比较逐事件整数几何、缩放、平移和指针，不以近似窗口代替原版

use super::SourcePreview;
use crate::backend::window::WindowCursor;

/// 【原图测试】【原版序列】解析实际组件的冻结事件并逐次比对原生状态，无参数、无返回值，不连接桌面
#[test]
fn actual_qt_image_geometry_and_pointer_sequences_match() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-ocr-source-preview.json"
    ))
    .unwrap();
    for scenario in fixture["scenarios"].as_array().unwrap() {
        let mut viewport = (
            scenario["viewport"][0].as_i64().unwrap() as i32,
            scenario["viewport"][1].as_i64().unwrap() as i32,
        );
        let image = (
            scenario["image"][0].as_u64().unwrap() as u32,
            scenario["image"][1].as_u64().unwrap() as u32,
        );
        let mut preview = SourcePreview::default();
        for (step, event) in scenario["events"].as_array().unwrap().iter().enumerate() {
            let point = || {
                (
                    event["point"][0].as_f64().unwrap(),
                    event["point"][1].as_f64().unwrap(),
                )
            };
            match event["kind"].as_str().unwrap() {
                "initial" => {}
                "wheel" => {
                    preview.wheel(viewport, image, point(), event["angle"].as_f64().unwrap());
                }
                "press" if event["button"] == 1 => {
                    preview.press(point());
                }
                "release" if event["button"] == 1 => {
                    preview.release();
                }
                "press" | "release" => {}
                "move" => {
                    preview.motion(viewport, image, point());
                }
                "double" => preview.reset(),
                "resize" => {
                    viewport = (
                        event["size"][0].as_i64().unwrap() as i32,
                        event["size"][1].as_i64().unwrap() as i32,
                    )
                }
                other => panic!("unknown fixture event {other}"),
            }
            let expected = &event["state"];
            let context = format!("{} step {step}", scenario["name"]);
            let rect: Vec<_> = preview
                .rect(viewport, image)
                .unwrap()
                .into_iter()
                .map(i64::from)
                .collect();
            assert_eq!(
                rect,
                expected["rect"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_i64().unwrap())
                    .collect::<Vec<_>>(),
                "{context}"
            );
            assert!(
                (preview.zoom - expected["zoom"].as_f64().unwrap()).abs() < 1e-12,
                "{context}"
            );
            assert_eq!(
                preview.pan,
                (
                    expected["pan"][0].as_f64().unwrap(),
                    expected["pan"][1].as_f64().unwrap()
                ),
                "{context}"
            );
            assert_eq!(
                preview.dragging,
                expected["dragging"].as_bool().unwrap(),
                "{context}"
            );
            let cursor = match preview.cursor() {
                WindowCursor::Default => 0,
                WindowCursor::Grab => 17,
                WindowCursor::Grabbing => 18,
                _ => unreachable!(),
            };
            assert_eq!(cursor, expected["cursor"].as_i64().unwrap(), "{context}");
        }
    }
}

/// 【原图测试】【输入安全】无效尺寸和非有限滚动不创建缓冲，失焦取消拖动但保留缩放，无参数、无返回值
#[test]
fn invalid_geometry_and_canceled_drag_do_not_allocate_or_reset_view() {
    let mut preview = SourcePreview::default();
    assert!(preview.rect((8, 8), (20, 20)).is_none());
    assert!(preview.rect((200, 200), (0, 20)).is_none());
    assert!(!preview.wheel((200, 200), (20, 20), (20.0, 20.0), f64::NAN));
    assert!(!preview.press((20.0, 20.0)));
    preview.wheel((200, 200), (20, 20), (20.0, 20.0), 120.0);
    preview.press((20.0, 20.0));
    assert!(preview.release());
    assert_eq!(preview.zoom, 1.12);
    assert!(preview.raster.is_none());
}

/// 【原图测试】【绘制资源】六倍放大仍只使用视区缓冲，原图透明度和父窗口外部像素保持不变，无参数、无返回值
#[test]
fn zoom_raster_is_viewport_bounded_and_transparent_pixels_do_not_gain_checkerboard() {
    use tiny_skia::{Color, Pixmap, Rect};
    let mut preview = SourcePreview::default();
    let image = Pixmap::new(100, 60).unwrap();
    let mut canvas = Pixmap::new(400, 300).unwrap();
    canvas.fill(Color::from_rgba8(15, 23, 42, 255));
    let before = canvas.data().to_vec();
    let body = Rect::from_xywh(8.0, 78.0, 300.0, 190.0).unwrap();
    preview.zoom = 6.0;
    super::super::source_preview_draw::draw(&mut preview, &image, &mut canvas, body, 1.0);
    assert_eq!(canvas.data(), before);
    assert_eq!(preview.raster.as_ref().unwrap().data().len(), 300 * 190 * 4);
    preview.zoom = 1.0;
    super::super::source_preview_draw::draw(&mut preview, &image, &mut canvas, body, 1.0);
    assert_eq!(preview.raster.as_ref().unwrap().data().len(), 300 * 190 * 4);
    assert_eq!(image.data(), vec![0; 100 * 60 * 4]);
}
