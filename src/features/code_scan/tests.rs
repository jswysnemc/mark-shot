//! 【扫码测试】【真实图像】覆盖多码、常见码制、透明背景、旋转和无效图像

use super::*;
use rxing::{BarcodeFormat, Writer};

/// 【扫码测试】【生成样本】按 format 和 text 创建真实黑白条码像素，返回 RGBA 图片
fn fixture(format: BarcodeFormat, text: &str) -> image::RgbaImage {
    let matrix = rxing::MultiFormatWriter
        .encode(text, &format, 320, 240)
        .unwrap();
    image::RgbaImage::from_fn(matrix.getWidth(), matrix.getHeight(), |x, y| {
        image::Rgba(if matrix.get(x, y) {
            [0, 0, 0, 255]
        } else {
            [255; 4]
        })
    })
}

/// 【扫码测试】【PNG编码】按 image 返回真实压缩图片，覆盖正式解码入口
fn png(image: image::RgbaImage) -> Vec<u8> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut buffer, image::ImageFormat::Png)
        .unwrap();
    buffer.into_inner()
}

/// 【扫码测试】【常见码制】无参数，验证二维码、Code128、EAN13、DataMatrix、Aztec 和 PDF417
#[test]
fn decodes_common_formats_without_external_runtime() {
    for (format, text) in [
        (BarcodeFormat::QR_CODE, "https://example.com/scan?x=1&y=2"),
        (BarcodeFormat::CODE_128, "Mark-Shot-128"),
        (BarcodeFormat::EAN_13, "5901234123457"),
        (BarcodeFormat::DATA_MATRIX, "Mark Shot Data Matrix"),
        (BarcodeFormat::AZTEC, "Mark Shot Aztec"),
        (BarcodeFormat::PDF_417, "Mark Shot PDF417"),
    ] {
        let result = decoder::decode(&png(fixture(format, text))).unwrap();
        assert!(
            result.iter().any(|code| code.text == text),
            "{format:?}: {result:?}"
        );
    }
}

/// 【扫码测试】【多码与转向】无参数，验证一张图片中的多个结果、中文内容和旋转图片
#[test]
fn finds_multiple_codes_and_preserves_unicode() {
    let first = fixture(BarcodeFormat::QR_CODE, "Mark Shot 扫码测试");
    let second = fixture(BarcodeFormat::QR_CODE, "https://example.com/second");
    let mut image = image::RgbaImage::from_pixel(720, 300, image::Rgba([255; 4]));
    image::imageops::overlay(&mut image, &first, 10, 30);
    image::imageops::overlay(&mut image, &second, 390, 30);
    let result = decoder::decode(&png(image::imageops::rotate90(&image))).unwrap();
    assert_eq!(result.len(), 2, "{result:?}");
    assert!(result.iter().any(|code| code.text == "Mark Shot 扫码测试"));
}

/// 【扫码测试】【透明与反色】无参数，验证透明留白和白码黑底可识别
#[test]
fn handles_transparent_and_inverted_codes() {
    let image = fixture(BarcodeFormat::QR_CODE, "transparent");
    let mut transparent = image.clone();
    for pixel in transparent.pixels_mut() {
        if pixel.0[0] == 255 {
            *pixel = image::Rgba([0, 0, 0, 0]);
        }
    }
    assert_eq!(
        decoder::decode(&png(transparent)).unwrap()[0].text,
        "transparent"
    );
    let mut inverted = image;
    image::imageops::invert(&mut inverted);
    assert_eq!(
        decoder::decode(&png(inverted)).unwrap()[0].text,
        "transparent"
    );
}

/// 【扫码测试】【混合极性】无参数，验证正常条码不能阻止同图反色 Data Matrix 的识别，旋转和反转后仍返回两种结果
#[test]
fn mixed_polarity_codes_survive_one_dimensional_crop_boundaries() {
    let fixture = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/code-scan/mixed-polarity.png"
    ));
    let mut image = image::load_from_memory(fixture).unwrap().into_rgba8();
    for inverted in [false, true] {
        for turn in 0..4 {
            let codes = decoder::decode(&png(image.clone())).unwrap();
            assert_eq!(
                codes.len(),
                2,
                "inverted={inverted}, turn={turn}: {codes:?}"
            );
            for text in ["Mark-Shot-128", "DM-SCAN-TEST"] {
                assert!(codes.iter().any(|code| code.text == text), "{codes:?}");
            }
            image = image::imageops::rotate90(&image);
        }
        image::imageops::invert(&mut image);
    }
}

/// 【扫码测试】【无结果与损坏】无参数，空白图返回空列表，损坏数据和空输入返回错误
#[test]
fn blank_and_invalid_inputs_are_distinct() {
    assert!(
        decoder::decode(&png(image::RgbaImage::from_pixel(
            128,
            128,
            image::Rgba([255; 4])
        )))
        .unwrap()
        .is_empty()
    );
    assert!(decoder::decode(b"invalid image").is_err());
    assert!(decoder::decode(&[]).is_err());
    assert!(transport::read(std::io::empty()).is_err());
}
