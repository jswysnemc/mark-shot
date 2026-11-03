use mark_shot::editor::file_canvas::FileCanvas;
use mark_shot::ocr::{OcrLine, OcrText, preprocess};
use tiny_skia::{Pixmap, Rect};

/// 【识别输入测试】【原始像素】验证缩小预览不降低 OCR 输入分辨率，透明底板按白色合成
#[test]
fn file_recognition_uses_original_pixels() {
    let mut pixels = Pixmap::new(1000, 500).unwrap();
    pixels.data_mut()[4..8].copy_from_slice(&[0, 0, 0, 255]);
    let file = FileCanvas::new(pixels, Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap()).unwrap();
    let prepared = preprocess::file_region(&file, file.viewport).unwrap();
    assert_eq!((prepared.image.width, prepared.image.height), (1000, 500));
    assert_eq!(&prepared.image.rgb[..6], &[255, 255, 255, 0, 0, 0]);
    assert!(prepared.scale > 1.0);
}

/// 【识别输入测试】【结果映射】验证文字框与每个字符边界使用相同的逻辑比例
#[test]
fn recognition_coordinates_follow_the_file_viewport() {
    let text = OcrText {
        lines: vec![OcrLine {
            text: "abc".into(),
            bounds: Rect::from_ltrb(100.0, 20.0, 400.0, 100.0).unwrap(),
            char_x: vec![100.0, 200.0, 300.0, 400.0],
        }],
    };
    let text = preprocess::normalize(text, 2.0);
    assert_eq!(text.lines[0].bounds.left(), 50.0);
    assert_eq!(text.lines[0].bounds.width(), 150.0);
    assert_eq!(text.lines[0].char_x, [50.0, 100.0, 150.0, 200.0]);
    assert_eq!(text.lines[0].text, "abc");
}
