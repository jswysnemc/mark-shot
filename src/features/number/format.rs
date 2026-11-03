//! 【序号】【标签格式】按原版规则转换七种编号样式，并限制极长罗马数字分配

use super::NumberStyle;

const MAX_ROMAN_LABEL: usize = 128;

impl NumberStyle {
    /// 【序号】【标签转换】将 number 转为当前样式文字；零和过长罗马数字使用十进制，返回有界字符串
    pub fn label(self, number: u32) -> String {
        if number == 0 {
            return "0".to_string();
        }
        match self {
            Self::Arabic => number.to_string(),
            Self::UpperAlpha => alphabetic(number, true),
            Self::LowerAlpha => alphabetic(number, false),
            Self::UpperRoman => roman(number, true),
            Self::LowerRoman => roman(number, false),
            Self::HeavenlyStem => {
                const STEMS: [char; 10] =
                    ['甲', '乙', '丙', '丁', '戊', '己', '庚', '辛', '壬', '癸'];
                STEMS[((number - 1) % 10) as usize].to_string()
            }
            Self::Chinese => chinese(number),
        }
    }
}

/// 【序号】【字母转换】将正整数 number 按二十六进位转换，uppercase 决定大小写；返回最长七个字母的标签
fn alphabetic(mut number: u32, uppercase: bool) -> String {
    let mut letters = Vec::with_capacity(7);
    let base = if uppercase { b'A' } else { b'a' };
    while number > 0 {
        number -= 1;
        letters.push(char::from(base + (number % 26) as u8));
        number /= 26;
    }
    letters.into_iter().rev().collect()
}

/// 【序号】【罗马转换】将正整数 number 转为减法罗马数字，uppercase 决定大小写；超过一百二十八字节时返回十进制
fn roman(number: u32, uppercase: bool) -> String {
    const PARTS: [(u32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut remaining = number;
    let mut label = String::with_capacity(16);
    for (value, text) in PARTS {
        let count = remaining / value;
        let bytes = count as usize * text.len();
        // 1. 【序号】【罗马转换】先检查重复次数，避免整数上限生成数百万个字符
        if bytes > MAX_ROMAN_LABEL.saturating_sub(label.len()) {
            return number.to_string();
        }
        for _ in 0..count {
            label.push_str(text);
        }
        remaining %= value;
    }
    if !uppercase {
        label.make_ascii_lowercase();
    }
    label
}

/// 【序号】【中文转换】将一到九千九百九十九的 number 转为中文，保留内部零；其他值返回十进制
fn chinese(number: u32) -> String {
    if !(1..=9999).contains(&number) {
        return number.to_string();
    }
    const DIGITS: [char; 10] = ['零', '一', '二', '三', '四', '五', '六', '七', '八', '九'];
    const UNITS: [(u32, &str); 4] = [(1000, "千"), (100, "百"), (10, "十"), (1, "")];
    let mut label = String::with_capacity(24);
    let mut zero = false;
    for (divisor, unit) in UNITS {
        let digit = (number / divisor) % 10;
        if digit == 0 {
            zero = !label.is_empty();
            continue;
        }
        if zero {
            label.push(DIGITS[0]);
            zero = false;
        }
        if !(digit == 1 && divisor == 10 && label.is_empty()) {
            label.push(DIGITS[digit as usize]);
        }
        label.push_str(unit);
    }
    label
}
