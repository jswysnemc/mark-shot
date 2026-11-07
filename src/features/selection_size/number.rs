//! 【选区尺寸】【整数验证】保留 Qt 英文整数字段的中间状态，分组文字可显示但确认仍使用普通十进制解析

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Validation {
    Invalid,
    Intermediate,
    Acceptable,
}

/// 【选区尺寸】【输入状态】按 text 和 maximum 返回原版英文整数验证状态，支持正号、分组分隔和暂时越界
pub fn validate(text: &str, maximum: i32) -> Validation {
    use Validation::{Acceptable, Intermediate, Invalid};
    let digits = text.strip_prefix('+').unwrap_or(text);
    if digits.is_empty() {
        return Intermediate;
    }
    if digits.starts_with(',')
        || digits.contains(",,")
        || !digits.bytes().all(|b| b.is_ascii_digit() || b == b',')
    {
        return Invalid;
    }
    let parts = digits.split(',').collect::<Vec<_>>();
    if parts.len() > 1
        && (parts.last().is_some_and(|p| p.len() > 3)
            || parts[1..parts.len() - 1].iter().any(|p| p.len() != 3))
    {
        return Invalid;
    }
    // 1. 【选区尺寸】【数值范围】验证阶段忽略分组符，确认阶段仍由字段普通解析决定是否应用
    let number = digits.replace(',', "");
    let Ok(value) = number.parse::<i32>() else {
        return Invalid;
    };
    let maximum = maximum.max(1);
    if value > maximum {
        return if number.len() <= maximum.to_string().len() {
            Intermediate
        } else {
            Invalid
        };
    }
    if value < 1 {
        return Intermediate;
    }
    // 2. 【选区尺寸】【分组状态】完整三位分组可确认，尚未完整的末组保留为中间输入
    if parts.len() == 1
        || ((1..=3).contains(&parts[0].len()) && parts[1..].iter().all(|p| p.len() == 3))
    {
        Acceptable
    } else {
        Intermediate
    }
}
