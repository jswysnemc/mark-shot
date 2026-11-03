//! 【截图历史测试】【PNG 信息】身份、元数据上限与像素保留

use super::super::{
    identity,
    png_metadata::{self, Prepared},
};
use std::io::Cursor;

/// 【截图历史测试】【严格身份】拒绝路径、非法日期、大小写标识和截断名称
#[test]
fn identity_requires_real_utc_date_and_plain_filename() {
    let valid = "capture-20261103123456123-0123456789abcdef0123456789abcdef.png";
    assert!(identity::timestamp(valid).is_some());
    for name in [
        format!("../{valid}"),
        format!("/{valid}"),
        valid.replace("20261103", "20260230"),
        valid.replace("123456123", "246056123"),
        valid.to_uppercase(),
        valid.replace(".png", ".jpg"),
    ] {
        assert!(identity::timestamp(&name).is_none(), "{name}");
    }
    let now = identity::timestamp(valid).unwrap();
    let next = identity::next(now, Some(now)).unwrap();
    assert_eq!(
        (identity::timestamp(&next).unwrap() - now).num_milliseconds(),
        1
    );
    let within_millisecond = now + chrono::Duration::microseconds(400);
    assert_eq!(
        (identity::timestamp(&identity::next(within_millisecond, Some(now)).unwrap()).unwrap()
            - now)
            .num_milliseconds(),
        1
    );
    assert!(identity::timestamp(&valid.replace("20261103", "00001103")).is_none());
    assert!(identity::timestamp(&valid.replace("123456123", "123460123")).is_none());
}

/// 【截图历史测试】【元数据无损】添加比例和中文搜索，保留原始 RGBA 与图像数据块
#[test]
fn metadata_roundtrip_keeps_pixels_and_compressed_data() {
    let original = super::png(17, 13);
    let mut recorded = Vec::new();
    Prepared::new(&original, 1.25, "会议 alpha\nnotes")
        .unwrap()
        .write(&mut recorded)
        .unwrap();
    let info = png_metadata::inspect(&mut Cursor::new(&recorded)).unwrap();
    assert_eq!(info.scale, 1.25);
    assert_eq!(info.search, "会议 alpha notes");
    assert_eq!(
        image::load_from_memory(&original).unwrap().into_rgba8(),
        image::load_from_memory(&recorded).unwrap().into_rgba8()
    );
    let idat = |bytes: &[u8]| {
        let mut offset = 8;
        let mut output = Vec::new();
        while offset < bytes.len() {
            let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
            if &bytes[offset + 4..offset + 8] == b"IDAT" {
                output.extend_from_slice(&bytes[offset..offset + length + 12]);
            }
            offset += length + 12;
        }
        output
    };
    assert_eq!(idat(&original), idat(&recorded));
    let mut second = Vec::new();
    Prepared::new(&recorded, f32::NAN, "")
        .unwrap()
        .write(&mut second)
        .unwrap();
    let second_info = png_metadata::inspect(&mut Cursor::new(second)).unwrap();
    assert_eq!(second_info.scale, 1.0);
    assert!(second_info.search.is_empty());
}

/// 【截图历史测试】【损坏边界】拒绝截断、坏校验和无图像数据，搜索按 UTF-8 边界限制长度
#[test]
fn rejects_corrupt_png_and_bounds_unicode_search() {
    let original = super::png(1, 1);
    for cut in [0, 8, 16, original.len() - 1, original.len() - 12] {
        assert!(png_metadata::inspect(&mut Cursor::new(&original[..cut])).is_err());
    }
    let mut corrupt = original.clone();
    corrupt[20] ^= 1;
    assert!(Prepared::new(&corrupt, 1.0, "").is_err());
    let text = png_metadata::bounded_search(&"汉".repeat(10_000));
    assert!(text.len() <= png_metadata::SEARCH_LIMIT);
    assert_eq!(text.len() % 3, 0);
    let large = "汉".repeat(100_000);
    let joined = png_metadata::joined_search(["meeting", large.as_str(), "not reached"]);
    assert!(joined.starts_with("meeting "));
    assert!(joined.len() <= png_metadata::SEARCH_LIMIT);
}
