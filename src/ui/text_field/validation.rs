//! 【字段编辑】【验证规则】候选不参与正文验证，确认整批校验，保留数值输入的有效中间状态

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rule {
    Text,
    Hex,
    Integer {
        min: i32,
        max: i32,
        suffix: &'static str,
    },
    SelectionSize {
        maximum: i32,
    },
    Number,
}

impl Rule {
    /// 【字段编辑】【显示后缀】返回规则的只读单位后缀，无参数；后缀同时用于原生周边文本
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Integer { suffix, .. } => suffix,
            _ => "",
        }
    }

    /// 【字段编辑】【长度上限】返回原版字段的 UTF-16 单元上限，无参数；不是 UTF-8 字节长度
    pub fn maximum(self) -> usize {
        if self == Self::Hex { 9 } else { 32767 }
    }

    /// 【字段编辑】【中间校验】按 text 返回是否允许保存正文，包括尚未完成的数值；非法混合字符整体拒绝
    pub fn accepts(self, text: &str) -> bool {
        let text = text.strip_suffix(self.suffix()).unwrap_or(text);
        match self {
            Self::Text => true,
            Self::Hex => {
                let hex = text.strip_prefix('#').unwrap_or(text);
                hex.len() <= 8 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
            }
            Self::Integer { min, max, .. } => {
                if text.is_empty() || (text == "-" && min < 0) {
                    return true;
                }
                let digits = if min < 0 {
                    text.strip_prefix('-').unwrap_or(text)
                } else {
                    text
                };
                !digits.is_empty()
                    && digits.bytes().all(|byte| byte.is_ascii_digit())
                    && text
                        .parse::<i32>()
                        .is_ok_and(|value| value <= max && value >= min.min(0))
            }
            Self::SelectionSize { maximum } => {
                crate::features::selection_size::number::validate(text, maximum)
                    != crate::features::selection_size::number::Validation::Invalid
            }
            Self::Number => {
                if matches!(text, "" | "-" | "." | "-.") {
                    return true;
                }
                let digits = text.strip_prefix('-').unwrap_or(text);
                digits
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'.')
                    && digits.bytes().filter(|byte| *byte == b'.').count() <= 1
                    && text.parse::<f32>().is_ok_and(f32::is_finite)
            }
        }
    }

    /// 【字段编辑】【完整校验】按 text 返回当前字段是否达到可确认状态，数值中间状态返回假
    pub fn acceptable(self, text: &str) -> bool {
        let text = text.strip_suffix(self.suffix()).unwrap_or(text);
        match self {
            Self::Integer { min, max, .. } => text
                .parse::<i32>()
                .is_ok_and(|value| (min..=max).contains(&value)),
            Self::SelectionSize { maximum } => {
                crate::features::selection_size::number::validate(text, maximum)
                    == crate::features::selection_size::number::Validation::Acceptable
            }
            Self::Number => text.parse::<f32>().is_ok_and(f32::is_finite),
            _ => self.accepts(text),
        }
    }

    /// 【字段编辑】【单位恢复】为 text 的有效纯数值补回缺失后缀，返回是否改写；非法残缺单位不修正
    pub(crate) fn restore_suffix(self, text: &mut String) -> bool {
        let suffix = self.suffix();
        if suffix.is_empty() || text.ends_with(suffix) || !self.accepts(text) {
            return false;
        }
        text.push_str(suffix);
        true
    }

    /// 【字段编辑】【确认裁切】按 allowance 返回 text 的合法 UTF-8 前缀，长度按 UTF-16 计；不会截断码点
    pub(crate) fn clipped(text: &str, allowance: usize) -> &str {
        let mut used = 0;
        for (byte, character) in text.char_indices() {
            if used + character.len_utf16() > allowance {
                return &text[..byte];
            }
            used += character.len_utf16();
        }
        text
    }
}
