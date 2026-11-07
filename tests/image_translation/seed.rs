//! 【图片翻译测试】【交接与身份】原图字节、已有识别、完整译文和图像坐标安全往返

use mark_shot::features::{
    image_translation::{
        BoxRect, Token, TranslatedBlock, TranslatedLine,
        seed::{self, Seed},
        selection,
    },
    pinned_image::translation::Translation,
};
use mark_shot::theme::Rgba;

/// 【图片翻译测试】【交接样本】返回完整译文与省略显示分开的 seed，原文已具备真实图像矩形
fn sample() -> Seed {
    Seed {
        source_tokens: vec![Token {
            text: "source".into(),
            rect: BoxRect::new(10.0, 10.0, 80.0, 20.0),
            line: 0,
            index: 0,
            confidence: 1.0,
        }],
        blocks: vec![TranslatedBlock {
            source: BoxRect::new(10.0, 10.0, 80.0, 20.0),
            plate: BoxRect::new(8.0, 8.0, 84.0, 30.0),
            background: Rgba(240, 242, 244, 255),
            foreground: Rgba(17, 24, 39, 255),
            font_size: 9.0,
            text: "Full translation remains available after visible truncation".into(),
            lines: vec![TranslatedLine {
                text: "可见…".into(),
                rect: BoxRect::new(10.0, 10.0, 20.0, 12.0),
                baseline: 19.0,
            }],
        }],
        show_translation: true,
    }
}

/// 【图片翻译测试】【字节往返】元数据还原全部坐标和全文，PNG 原始字节仍借用传输帧，不重新编码或复制
#[test]
fn seed_roundtrip_preserves_metadata_full_text_and_original_png_bytes() {
    let seed = sample();
    let png = b"original-png-bytes";
    let encoded = seed::encode(png, &seed).unwrap();
    let (decoded, borrowed) = seed::decode(&encoded).unwrap();
    assert_eq!(decoded, seed);
    assert_eq!(borrowed, png);
    assert!(borrowed.as_ptr() as usize >= encoded.as_ptr() as usize);
    assert!(
        borrowed.as_ptr() as usize + borrowed.len() <= encoded.as_ptr() as usize + encoded.len()
    );
    for length in [
        0,
        1,
        17,
        20,
        encoded.len() - png.len() - 1,
        encoded.len() - png.len(),
    ] {
        assert!(
            seed::decode(&encoded[..length]).is_err(),
            "truncated length {length}"
        );
    }
}

/// 【图片翻译测试】【范围交接】识别框裁切到新原图，译文保留可相交底板及全文，基线和行框同步平移
#[test]
fn shifted_seed_clips_source_tokens_and_preserves_intersecting_translation() {
    let seed = sample().shifted((-20.0, 5.0), BoxRect::new(0.0, 0.0, 50.0, 60.0));
    assert_eq!(
        seed.source_tokens[0].rect,
        BoxRect::new(0.0, 15.0, 50.0, 20.0)
    );
    assert_eq!(seed.blocks[0].plate, BoxRect::new(-12.0, 13.0, 84.0, 30.0));
    assert_eq!(seed.blocks[0].lines[0].baseline, 24.0);
    assert_eq!(seed.blocks[0].text, sample().blocks[0].text);
    assert!(
        sample()
            .shifted((1000.0, 0.0), BoxRect::new(0.0, 0.0, 100.0, 100.0))
            .blocks
            .is_empty()
    );
}

/// 【图片翻译测试】【非法几何】无效字号、非有限基线和识别坐标不能进入显示或任务，错误不产生部分传输帧
#[test]
fn invalid_geometry_cannot_be_encoded_as_a_translation_seed() {
    let mut seed = sample();
    seed.blocks[0].font_size = f32::INFINITY;
    assert!(seed::encode(b"png", &seed).is_err());
    let mut seed = sample();
    seed.blocks[0].lines[0].baseline = f64::NAN;
    assert!(seed::encode(b"png", &seed).is_err());
    let mut seed = sample();
    seed.source_tokens[0].rect.0[2] = 0.0;
    assert!(seed::encode(b"png", &seed).is_err());
}

/// 【图片翻译测试】【任务身份】目标变更解除旧结果，失败保留当前覆盖层，明确成功才替换当前译文
#[test]
fn language_changes_reject_old_tasks_and_empty_results_preserve_the_display() {
    let mut state = Translation::new("English".into());
    state.blocks = sample().blocks;
    state.active = true;
    state.pending = Some(7);
    let original = state.blocks.clone();
    assert!(!state.complete(7, Vec::new()));
    assert_eq!(state.blocks, original);
    assert!(state.active);
    state.pending = Some(8);
    assert!(state.set_target("Japanese".into()));
    assert!(!state.complete(8, original.clone()));
    assert!(state.blocks.is_empty());
    assert!(!state.active);
    state.pending = Some(9);
    assert!(state.complete(9, original));
    assert!(state.active);
    state.clear();
    assert!(state.blocks.is_empty());
    assert_eq!(state.target, "Japanese");
}

/// 【图片翻译测试】【字符列宽】中文占两列、组合字符零宽，显示省略行独立选择，原始全文不受影响
#[test]
fn selectable_lines_keep_wide_character_weights_and_original_full_text() {
    let mut seed = sample();
    seed.blocks[0].lines[0].text = "中ae\u{301}".into();
    let lines = selection::lines(&seed.blocks);
    assert_eq!(lines[0].char_x, vec![10.0, 20.0, 25.0, 30.0, 30.0]);
    assert_eq!(lines[0].text, "中ae\u{301}");
    assert_eq!(seed.blocks[0].text, sample().blocks[0].text);
}
