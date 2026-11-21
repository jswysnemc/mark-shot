//! 【RTF解析】【代码页】把 `\'hh` 字节按 `\ansicpgN` 声明的代码页解码为文本

use encoding_rs::{BIG5, EUC_KR, Encoding, GBK, SHIFT_JIS, UTF_8, WINDOWS_1250, WINDOWS_1252};

/// 【RTF解析】【代码页解码】按 codepage 选择编码器并把 bytes 解码为字符串，未知代码页回退 windows-1252
pub(super) fn decode(codepage: u16, bytes: &[u8]) -> String {
    let encoding = encoding_for(codepage);
    encoding.decode(bytes).0.into_owned()
}

/// 【RTF解析】【编码器选择】按 codepage 返回 encoding_rs 编码器，覆盖常见东亚与西欧代码页
fn encoding_for(codepage: u16) -> &'static Encoding {
    match codepage {
        65001 => UTF_8,
        936 => GBK,
        950 => BIG5,
        932 => SHIFT_JIS,
        949 => EUC_KR,
        1250 => WINDOWS_1250,
        1251 => encoding_rs::WINDOWS_1251,
        // 28591 是 ISO-8859-1 的官方代码页，encoding_rs 用 windows-1252 作为其超集实现
        1252 | 28591 | 20127 => WINDOWS_1252,
        1253 => encoding_rs::WINDOWS_1253,
        1254 => encoding_rs::WINDOWS_1254,
        1255 => encoding_rs::WINDOWS_1255,
        1256 => encoding_rs::WINDOWS_1256,
        1257 => encoding_rs::WINDOWS_1257,
        1258 => encoding_rs::WINDOWS_1258,
        _ => WINDOWS_1252,
    }
}

#[cfg(test)]
mod tests {
    use super::decode;

    /// 【RTF测试】【国标解码】按 ansicpg936 解码两个 GBK 字节，返回汉字“中”
    #[test]
    fn gbk_bytes_decode_to_chinese() {
        assert_eq!(decode(936, &[0xd6, 0xd0]), "中");
    }

    /// 【RTF测试】【UTF8解码】按 ansicpg65001 解码 UTF-8 字节，返回与源文本一致
    #[test]
    fn utf8_bytes_round_trip() {
        assert_eq!(decode(65001, "中".as_bytes()), "中");
    }

    /// 【RTF测试】【默认代码页】按未知代码页解码 ASCII，返回原字符
    #[test]
    fn unknown_codepage_falls_back_to_western() {
        assert_eq!(decode(1, b"abc"), "abc");
    }
}
