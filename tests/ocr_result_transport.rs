use mark_shot::{
    features::ocr_result::{document::OcrDocument, transport},
    ocr::OcrLine,
};
use tiny_skia::Rect;

/// 【结果测试】【文档构造】返回包含中文、负坐标与原图字节的结果文档
fn document() -> OcrDocument {
    OcrDocument {
        text: "识别内容\nnext".into(),
        png: vec![1, 2, 3, 4],
        lines: vec![OcrLine {
            text: "识别内容".into(),
            bounds: Rect::from_xywh(-10.0, 2.5, 80.0, 16.0).unwrap(),
            char_x: vec![],
        }],
    }
}

/// 【结果测试】【二进制往返】验证 Unicode、坐标与原图字节完整传递
#[test]
fn result_frame_round_trips() {
    let document = document();
    let mut encoded = Vec::new();
    transport::write(&document, &mut encoded).unwrap();
    let decoded = transport::read(encoded.as_slice()).unwrap();
    assert_eq!(decoded.text, document.text);
    assert_eq!(decoded.png, document.png);
    assert_eq!(decoded.lines, document.lines);
}

/// 【结果测试】【截断拒绝】验证每个截断位置都会返回错误而非部分文档
#[test]
fn truncated_frames_are_rejected() {
    let mut encoded = Vec::new();
    transport::write(&document(), &mut encoded).unwrap();
    for end in 0..encoded.len() {
        assert!(
            transport::read(&encoded[..end]).is_err(),
            "accepted prefix {end}"
        );
    }
}

/// 【结果测试】【大小校验】验证读取像素前拒绝损坏标识和过大长度
#[test]
fn invalid_headers_do_not_allocate_images() {
    let mut encoded = Vec::new();
    transport::write(&document(), &mut encoded).unwrap();
    encoded[0] = b'X';
    assert!(transport::read(encoded.as_slice()).is_err());
    encoded[0] = b'M';
    encoded[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(transport::read(encoded.as_slice()).is_err());
}
