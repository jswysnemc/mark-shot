//! 【RTF解析】【词法】把 RTF 源码切分为分组、控制字、控制符号、十六进制字节与纯文本

/// 【RTF解析】【记号】描述一个 RTF 词法单元
pub(super) enum Token<'a> {
    /// 左花括号，开始一个分组
    Start,
    /// 右花括号，结束一个分组
    End,
    /// 控制字及其可选数字参数
    Control(&'a [u8], Option<i32>),
    /// 反斜杠后的单个控制符号
    Symbol(u8),
    /// `\'hh` 表示的单个字节
    Hex(u8),
    /// 一段连续纯文本字节
    Text(&'a [u8]),
}

/// 【RTF解析】【词法器】记录源码字节与扫描位置，按需产生记号
pub(super) struct Lexer<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// 【RTF解析】【初始化】按 data 创建词法器，扫描位置从零开始
    pub(super) fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// 【RTF解析】【跳过字节】按 bytes 前进扫描位置，用于 `\binN`，越界时停在末尾
    pub(super) fn skip(&mut self, bytes: usize) {
        self.pos = self.pos.saturating_add(bytes).min(self.data.len());
    }

    /// 【RTF解析】【下一记号】无参数，返回下一个记号；源码耗尽时返回空值
    pub(super) fn next(&mut self) -> Option<Token<'a>> {
        loop {
            let byte = *self.data.get(self.pos)?;
            match byte {
                b'{' => {
                    self.pos += 1;
                    return Some(Token::Start);
                }
                b'}' => {
                    self.pos += 1;
                    return Some(Token::End);
                }
                b'\\' => return self.control(),
                b'\r' | b'\n' => self.pos += 1,
                _ => {
                    let start = self.pos;
                    while let Some(current) = self.data.get(self.pos) {
                        if matches!(*current, b'{' | b'}' | b'\\' | b'\r' | b'\n') {
                            break;
                        }
                        self.pos += 1;
                    }
                    return Some(Token::Text(&self.data[start..self.pos]));
                }
            }
        }
    }

    /// 【RTF解析】【反斜杠记号】按当前位置解析控制字或控制符号，返回相应记号
    fn control(&mut self) -> Option<Token<'a>> {
        self.pos += 1;
        let byte = *self.data.get(self.pos)?;
        if byte.is_ascii_alphabetic() {
            let start = self.pos;
            while self.data.get(self.pos).is_some_and(u8::is_ascii_alphabetic) {
                self.pos += 1;
            }
            let word = &self.data[start..self.pos];
            let mut negative = false;
            if self.data.get(self.pos) == Some(&b'-') {
                negative = true;
                self.pos += 1;
            }
            let digit_start = self.pos;
            let mut value: i64 = 0;
            while let Some(current) = self.data.get(self.pos) {
                if !current.is_ascii_digit() {
                    break;
                }
                value = value
                    .saturating_mul(10)
                    .saturating_add(i64::from(current - b'0'))
                    .min(i64::from(i32::MAX));
                self.pos += 1;
            }
            let param = (self.pos > digit_start).then(|| {
                let signed = if negative { -value } else { value };
                signed.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
            });
            if self.data.get(self.pos) == Some(&b' ') {
                self.pos += 1;
            }
            return Some(Token::Control(word, param));
        }
        self.pos += 1;
        if byte == b'\'' {
            let high = self.data.get(self.pos).and_then(hex_digit);
            let low = self.data.get(self.pos + 1).and_then(hex_digit);
            if let (Some(high), Some(low)) = (high, low) {
                self.pos += 2;
                return Some(Token::Hex(high * 16 + low));
            }
            return Some(Token::Symbol(b'\''));
        }
        Some(Token::Symbol(byte))
    }
}

/// 【RTF解析】【十六进制】按 byte 解析一个十六进制数字，非法字符返回空值
fn hex_digit(byte: &u8) -> Option<u8> {
    (*byte as char).to_digit(16).map(|value| value as u8)
}
