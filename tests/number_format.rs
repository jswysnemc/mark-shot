//! 【序号测试】【格式转换】验证七种样式、跨位进位和有界罗马数字标签

use mark_shot::features::number::{NumberSequence, NumberStyle};

/// 【序号测试】【样式标识】七种样式具有独立、稳定的配置名称，未知名称不回退到其他样式
#[test]
fn every_style_has_a_stable_configuration_key() {
    assert_eq!(NumberStyle::ALL.len(), 7);
    for style in NumberStyle::ALL {
        assert_eq!(NumberStyle::parse(style.key()), Some(style));
        assert_eq!(
            NumberStyle::parse(&style.key().to_ascii_uppercase()),
            Some(style)
        );
        let encoded = serde_json::to_string(&style).unwrap();
        assert_eq!(
            serde_json::from_str::<NumberStyle>(&encoded).unwrap(),
            style
        );
    }
    assert_eq!(NumberStyle::parse("unknown"), None);
    assert!(serde_json::from_str::<NumberStyle>("\"unknown\"").is_err());
}

/// 【序号测试】【字母进位】字母采用 A 到 Z 后继续 AA 的规则，不在二十六项后重复
#[test]
fn alphabetic_labels_use_spreadsheet_style_carrying() {
    for (number, expected) in [
        (1, "A"),
        (26, "Z"),
        (27, "AA"),
        (52, "AZ"),
        (53, "BA"),
        (703, "AAA"),
    ] {
        assert_eq!(NumberStyle::UpperAlpha.label(number), expected);
        assert_eq!(
            NumberStyle::LowerAlpha.label(number),
            expected.to_ascii_lowercase()
        );
    }
    assert_eq!(NumberStyle::UpperAlpha.label(u32::MAX), "MWLQKWU");
}

/// 【序号测试】【罗马规则】罗马数字使用减法形式，正常范围保持原版规则，超长标签回退到十进制
#[test]
fn roman_labels_support_subtractive_forms_without_unbounded_allocation() {
    for (number, expected) in [
        (1, "I"),
        (4, "IV"),
        (9, "IX"),
        (49, "XLIX"),
        (944, "CMXLIV"),
        (3999, "MMMCMXCIX"),
        (4001, "MMMMI"),
    ] {
        assert_eq!(NumberStyle::UpperRoman.label(number), expected);
        assert_eq!(
            NumberStyle::LowerRoman.label(number),
            expected.to_ascii_lowercase()
        );
    }
    assert_eq!(
        NumberStyle::UpperRoman.label(u32::MAX),
        u32::MAX.to_string()
    );
    assert!(NumberStyle::UpperRoman.label(127_999).len() <= 128);
}

/// 【序号测试】【中文零位】中文保留内部零、省略十位开头的一，并在一万以上沿用原版十进制回退
#[test]
fn chinese_labels_preserve_internal_zeroes_and_original_large_number_fallback() {
    for (number, expected) in [
        (1, "一"),
        (10, "十"),
        (11, "十一"),
        (20, "二十"),
        (101, "一百零一"),
        (110, "一百一十"),
        (1001, "一千零一"),
        (1010, "一千零一十"),
        (1100, "一千一百"),
        (9999, "九千九百九十九"),
        (10000, "10000"),
    ] {
        assert_eq!(NumberStyle::Chinese.label(number), expected);
    }
}

/// 【序号测试】【天干循环】天干以十项循环，零在所有样式下显示十进制而不产生下溢
#[test]
fn heavenly_stems_cycle_and_zero_is_safe_for_every_style() {
    for (number, expected) in [(1, "甲"), (3, "丙"), (10, "癸"), (11, "甲"), (20, "癸")] {
        assert_eq!(NumberStyle::HeavenlyStem.label(number), expected);
    }
    for style in NumberStyle::ALL {
        assert_eq!(style.label(0), "0");
    }
}

/// 【序号测试】【连续计数】提交后推进下一编号，重置可恢复一，不根据现有标注空缺重新编号
#[test]
fn sequence_progression_is_explicit_and_can_be_reset() {
    let mut sequence = NumberSequence::default();
    assert_eq!(sequence.next(), Some(1));
    sequence.advance(1);
    assert_eq!(sequence.next(), Some(2));
    sequence.advance(8);
    assert_eq!(sequence.next(), Some(9));
    sequence.advance(3);
    assert_eq!(sequence.next(), Some(9));
    assert!(sequence.reset());
    assert_eq!(sequence.next(), Some(1));
    assert!(!sequence.reset());
}

/// 【序号测试】【计数耗尽】最大编号提交后停止创建，不溢出、不重复，重置后重新允许编号
#[test]
fn exhausted_sequences_never_wrap_or_silently_repeat() {
    let mut sequence = NumberSequence::default();
    sequence.advance(u32::MAX);
    assert_eq!(sequence.next(), None);
    sequence.advance(1);
    assert_eq!(sequence.next(), None);
    assert!(sequence.reset());
    assert_eq!(sequence.next(), Some(1));
}
