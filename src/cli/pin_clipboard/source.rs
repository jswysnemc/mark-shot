//! 【贴图】【剪贴板来源】读取原生图片、文件列表、HTML 与文本，所有缓冲都有上限

use super::{IMAGE_LIMIT, MARKUP_LIMIT, TEXT_LIMIT};
use std::path::PathBuf;

#[derive(Default)]
pub(super) struct Clipboard {
    pub image: Option<Vec<u8>>,
    pub files: Vec<PathBuf>,
    pub html: Option<String>,
    pub rtf: Option<Vec<u8>>,
    pub text: Option<String>,
    pub sequence: u32,
}

impl Clipboard {
    /// 【贴图】【重复检测】无参数，返回剪贴板是否提供可区分新复制操作的序号；Wayland 返回 false
    pub fn has_sequence(&self) -> bool {
        self.sequence != 0
    }

    /// 【贴图】【重复身份】无参数，返回文件列表及可用剪贴板版本的摘要，不持久化文件路径
    pub fn fingerprint(&self) -> u32 {
        let mut hash = crc32fast::Hasher::new();
        hash.update(&self.sequence.to_le_bytes());
        for path in &self.files {
            hash.update(path.to_string_lossy().as_bytes());
            hash.update(&[0]);
        }
        hash.finalize()
    }
}

/// 【贴图】【URI文件列表】按 text 解析复制文件的 URI 列表，忽略注释及远程非文件协议，返回本地路径
#[cfg(any(target_os = "linux", test))]
pub(super) fn files(text: &str) -> Vec<PathBuf> {
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| url::Url::parse(line.trim()).ok()?.to_file_path().ok())
        .take(256)
        .collect()
}

/// 【贴图】【HTML类型排序】按 mimes 过滤 HTML 基础类型，优先 UTF-8 后按字符集排序，返回原始字符串列表
#[cfg(any(target_os = "linux", test))]
fn html_mime_candidates(mimes: &[String]) -> Vec<String> {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    let mut candidates: Vec<(u8, String)> = Vec::new();
    for mime in mimes {
        let base = mime.split(';').next().unwrap_or(mime).trim();
        if !base.eq_ignore_ascii_case("text/html")
            && !base.eq_ignore_ascii_case("application/xhtml+xml")
        {
            continue;
        }
        if !seen.insert(mime.trim().to_ascii_lowercase()) {
            continue;
        }
        let rank = match html_charset(mime).as_deref() {
            None | Some("utf-8" | "utf8") => 0,
            Some("us-ascii" | "ascii") => 1,
            Some("utf-16le") => 2,
            Some("utf-16be") => 3,
            Some("utf-16") => 4,
            Some("iso-8859-1" | "latin1") => 5,
            Some(_) => 6,
        };
        candidates.push((rank, mime.clone()));
    }
    candidates.sort_by_key(|(rank, _)| *rank);
    candidates.into_iter().map(|(_, mime)| mime).collect()
}

/// 【贴图】【HTML字符集】按 mime 解析 charset 参数，去掉引号并转小写，缺少参数时返回空值
#[cfg(any(target_os = "linux", test))]
fn html_charset(mime: &str) -> Option<String> {
    mime.split(';').skip(1).find_map(|part| {
        let (key, value) = part.split_once('=')?;
        if !key.trim().eq_ignore_ascii_case("charset") {
            return None;
        }
        let value = value.trim_matches(['"', '\'']).trim();
        (!value.is_empty()).then(|| value.to_ascii_lowercase())
    })
}

/// 【贴图】【HTML解码】按 bytes 与声明的 mime 字符集转换文本，移除 BOM，空内容或无效编码返回空值
#[cfg(any(target_os = "linux", test))]
fn decode_html(bytes: &[u8], mime: &str) -> Option<String> {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match html_charset(mime).as_deref() {
        Some("utf-8" | "utf8" | "us-ascii" | "ascii") => decode_utf8(bytes),
        Some("utf-16le") => decode_utf16(bytes, true),
        Some("utf-16be") => decode_utf16(bytes, false),
        Some("utf-16") => {
            if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
                decode_utf16(rest, true)
            } else if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
                decode_utf16(rest, false)
            } else {
                decode_utf16(bytes, true)
            }
        }
        Some("iso-8859-1" | "latin1") => {
            finish(bytes.iter().map(|byte| char::from(*byte)).collect())
        }
        Some(_) => decode_utf8(bytes).or_else(|| decode_utf16(bytes, true)),
        None => decode_utf8(bytes).or_else(|| {
            if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
                decode_utf16(rest, true)
            } else if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
                decode_utf16(rest, false)
            } else {
                None
            }
        }),
    }
}

/// 【贴图】【UTF8解码】按字节严格转换为文本并移除首尾 BOM，无效编码或空内容返回空值
#[cfg(any(target_os = "linux", test))]
fn decode_utf8(bytes: &[u8]) -> Option<String> {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).ok()?;
    finish(text.to_string())
}

/// 【贴图】【UTF16解码】按字节与小端标志转换文本，忽略末尾奇数字节和零终止，空内容返回空值
#[cfg(any(target_os = "linux", test))]
fn decode_utf16(bytes: &[u8], little_endian: bool) -> Option<String> {
    let units = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            if little_endian {
                u16::from_le_bytes([pair[0], pair[1]])
            } else {
                u16::from_be_bytes([pair[0], pair[1]])
            }
        })
        .take_while(|unit| *unit != 0)
        .collect::<Vec<_>>();
    String::from_utf16(&units).ok().and_then(finish)
}

/// 【贴图】【文本收尾】按 text 移除首位 U+FEFF，空结果返回空值
#[cfg(any(target_os = "linux", test))]
fn finish(text: String) -> Option<String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    (!text.is_empty()).then(|| text.to_string())
}

/// 【贴图】【RTF类型排序】按 mimes 过滤 RTF 基础类型，优先 text/rtf 后 application/rtf，返回原始字符串列表
#[cfg(any(target_os = "linux", test))]
fn rtf_mime_candidates(mimes: &[String]) -> Vec<String> {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    let mut candidates: Vec<(u8, String)> = Vec::new();
    for mime in mimes {
        let base = mime.split(';').next().unwrap_or(mime).trim();
        let rank = if base.eq_ignore_ascii_case("text/rtf") {
            0
        } else if base.eq_ignore_ascii_case("application/rtf") {
            1
        } else {
            continue;
        };
        if !seen.insert(mime.trim().to_ascii_lowercase()) {
            continue;
        }
        candidates.push((rank, mime.clone()));
    }
    candidates.sort_by_key(|(rank, _)| *rank);
    candidates.into_iter().map(|(_, mime)| mime).collect()
}

/// 【贴图】【RTF来源保留】按 bytes 移除 UTF-8 BOM 和末尾终止符，返回原始字节；代码页由 RTF 解析器决定，空内容返回空值
#[cfg(any(target_os = "linux", test))]
fn decode_rtf_source(bytes: &[u8]) -> Option<Vec<u8>> {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let end = bytes
        .iter()
        .rposition(|byte| *byte != 0)
        .map_or(0, |end| end + 1);
    let bytes = &bytes[..end];
    bytes
        .iter()
        .any(|byte| !byte.is_ascii_whitespace())
        .then(|| bytes.to_vec())
}

/// 【贴图】【Linux类型读取】按 x11、mime、limit 读取有界数据，缺少类型或权限时返回错误
#[cfg(target_os = "linux")]
fn data_with(
    x11: bool,
    mime: wl_clipboard_rs::paste::MimeType<'_>,
    limit: u64,
) -> crate::AppResult<Vec<u8>> {
    use std::io::Read;
    use wl_clipboard_rs::paste::{ClipboardType, MimeType, Seat, get_contents};
    if x11 {
        let target = match mime {
            MimeType::Specific(target) => target,
            _ => "UTF8_STRING",
        };
        return crate::backend::x11::clipboard::read(target, false, limit as usize);
    }
    let (reader, _) = get_contents(ClipboardType::Regular, Seat::Unspecified, mime)?;
    let mut bytes = Vec::new();
    reader.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("clipboard content exceeds the pin limit".into());
    }
    Ok(bytes)
}

/// 【贴图】【Linux剪贴板】无参数，优先识别复制文件，再读取图片、HTML 或纯文本，返回可用内容
#[cfg(target_os = "linux")]
pub(super) fn read() -> crate::AppResult<Clipboard> {
    use wl_clipboard_rs::paste::{ClipboardType, MimeType, Seat, get_mime_types_ordered};
    let x11 = crate::backend::x11::clipboard::use_x11()?;
    let file_data = data_with(x11, MimeType::Specific("text/uri-list"), TEXT_LIMIT).or_else(|_| {
        data_with(
            x11,
            MimeType::Specific("x-special/gnome-copied-files"),
            TEXT_LIMIT,
        )
    });
    let files = file_data
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map_or_else(Vec::new, |text| self::files(&text));
    // 1. 【贴图】【文件图片回退】复制文件时也读取图片预览，文件不可解码时仍能贴出原图
    let image = data_with(x11, MimeType::Specific("image/png"), IMAGE_LIMIT).ok();
    if !files.is_empty() {
        return Ok(Clipboard {
            files,
            image,
            sequence: x11_sequence(x11).unwrap_or(0),
            ..Default::default()
        });
    }
    let (html, rtf, text) = if image.is_none() {
        let mimes = if x11 {
            crate::backend::x11::clipboard::targets(false).unwrap_or_default()
        } else {
            get_mime_types_ordered(ClipboardType::Regular, Seat::Unspecified).unwrap_or_default()
        };
        let mut candidates = html_mime_candidates(&mimes);
        if x11 && candidates.is_empty() {
            candidates.push("text/html".into());
        }
        let html = candidates.iter().find_map(|mime| {
            data_with(x11, MimeType::Specific(mime), MARKUP_LIMIT)
                .ok()
                .and_then(|bytes| decode_html(&bytes, mime))
        });
        let rtf = if html.is_none() {
            let mut rtf_candidates = rtf_mime_candidates(&mimes);
            if x11 && rtf_candidates.is_empty() {
                rtf_candidates.push("text/rtf".into());
                rtf_candidates.push("application/rtf".into());
            }
            rtf_candidates.iter().find_map(|mime| {
                data_with(x11, MimeType::Specific(mime), MARKUP_LIMIT)
                    .ok()
                    .and_then(|bytes| decode_rtf_source(&bytes))
            })
        } else {
            None
        };
        let text = data_with(x11, MimeType::Text, TEXT_LIMIT)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok());
        (html, rtf, text)
    } else {
        (None, None, None)
    };
    Ok(Clipboard {
        image,
        html,
        rtf,
        text,
        ..Default::default()
    })
}

/// 【贴图】【X11剪贴板版本】按 x11 返回选择拥有者编号；Wayland 或无连接时返回空值
#[cfg(target_os = "linux")]
fn x11_sequence(x11: bool) -> Option<u32> {
    use x11rb::protocol::xproto::ConnectionExt;
    if !x11 {
        return None;
    }
    let (connection, _) = x11rb::connect(None).ok()?;
    let atom = connection
        .intern_atom(false, b"CLIPBOARD")
        .ok()?
        .reply()
        .ok()?
        .atom;
    Some(
        connection
            .get_selection_owner(atom)
            .ok()?
            .reply()
            .ok()?
            .owner,
    )
}

/// 【贴图】【Windows剪贴板】无参数，读取 CF_HDROP、PNG/DIB、HTML 或 Unicode，并记录系统剪贴板序号
#[cfg(target_os = "windows")]
pub(super) fn read() -> crate::AppResult<Clipboard> {
    let files = crate::backend::clipboard::read_files().unwrap_or_default();
    let sequence =
        unsafe { windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber() };
    // 1. 【贴图】【文件图片回退】复制文件时也读取图片预览，文件不可解码时仍能贴出原图
    let image = crate::backend::clipboard::read_image_png()
        .ok()
        .filter(|bytes| bytes.len() as u64 <= IMAGE_LIMIT);
    if !files.is_empty() {
        return Ok(Clipboard {
            files,
            image,
            sequence,
            ..Default::default()
        });
    }
    let html = crate::backend::clipboard::read_html().ok();
    let rtf = if html.is_none() {
        crate::backend::clipboard::read_rtf()
            .ok()
            .filter(|text| text.len() as u64 <= MARKUP_LIMIT)
    } else {
        None
    };
    let text = crate::backend::clipboard::read_text()
        .ok()
        .filter(|text| text.len() as u64 <= TEXT_LIMIT);
    Ok(Clipboard {
        image,
        html,
        rtf,
        text,
        sequence,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::{
        decode_html, decode_rtf_source, html_charset, html_mime_candidates, rtf_mime_candidates,
    };

    /// 【贴图测试】【UTF16LE编码】按 text 返回小端 UTF-16 字节，测试用
    fn utf16le(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    /// 【贴图测试】【UTF16BE编码】按 text 返回大端 UTF-16 字节，测试用
    fn utf16be(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_be_bytes).collect()
    }

    /// 【贴图测试】【HTML类型排序】无参数，验证 UTF-8 优先、字符集排序、去重并保留完整参数串
    #[test]
    fn html_mime_candidates_rank_utf8_then_utf16_and_iso() {
        let mimes = [
            "text/html;charset=UTF-16LE",
            "text/html;charset=ISO-8859-1",
            "text/plain",
            "text/html",
            "application/xhtml+xml;charset=utf-16be",
            "text/html;charset=utf-8",
            "text/html;charset=UTF-16",
            "text/rtf",
            "image/png",
            "text/html;charset=UTF-16LE",
        ]
        .map(str::to_string);
        assert_eq!(
            html_mime_candidates(&mimes),
            [
                "text/html",
                "text/html;charset=utf-8",
                "text/html;charset=UTF-16LE",
                "application/xhtml+xml;charset=utf-16be",
                "text/html;charset=UTF-16",
                "text/html;charset=ISO-8859-1",
            ]
        );
    }

    /// 【贴图测试】【字符集参数】无参数，验证引号、大小写与缺失参数的处理
    #[test]
    fn html_charset_strips_quotes_and_lowercases() {
        assert_eq!(
            html_charset("text/html; charset=\"UTF-16LE\"").as_deref(),
            Some("utf-16le")
        );
        assert_eq!(
            html_charset("text/html;charset='ISO-8859-1'").as_deref(),
            Some("iso-8859-1")
        );
        assert_eq!(html_charset("text/html"), None);
        assert_eq!(html_charset("text/html;charset="), None);
    }

    /// 【贴图测试】【UTF16LE】无参数，验证 IDEA 风格 HTML（含中文）按声明字符集正确解码
    #[test]
    fn decode_html_reads_utf16le_ides_with_cjk() {
        let html = "<html><body><div style=\"background-color:#1e1f22\">代码</div></body></html>";
        let bytes = utf16le(html);
        assert_eq!(
            decode_html(&bytes, "text/html;charset=UTF-16LE").as_deref(),
            Some(html)
        );
        assert_eq!(
            decode_html(&bytes, "text/html;charset=utf-16le").as_deref(),
            Some(html)
        );
    }

    /// 【贴图测试】【UTF16BE与BOM】无参数，验证大端、UTF-16 双 BOM 与奇数字节容忍
    #[test]
    fn decode_html_reads_utf16_endianness_and_boms() {
        let text = "café 代码";
        assert_eq!(
            decode_html(&utf16be(text), "text/html;charset=utf-16be").as_deref(),
            Some(text)
        );
        let mut little = vec![0xFF, 0xFE];
        little.extend(utf16le(text));
        assert_eq!(
            decode_html(&little, "text/html;charset=utf-16").as_deref(),
            Some(text)
        );
        let mut big = vec![0xFE, 0xFF];
        big.extend(utf16be(text));
        assert_eq!(
            decode_html(&big, "text/html;charset=utf-16").as_deref(),
            Some(text)
        );
        let mut odd = utf16le("ab");
        odd.push(0x41);
        assert_eq!(
            decode_html(&odd, "text/html;charset=utf-16le").as_deref(),
            Some("ab")
        );
    }

    /// 【贴图测试】【UTF8与单字节】无参数，验证 UTF-8 BOM、ISO-8859-1 与 US-ASCII
    #[test]
    fn decode_html_reads_utf8_latin1_and_ascii() {
        let mut bom = vec![0xEF, 0xBB, 0xBF];
        bom.extend("代码".as_bytes());
        assert_eq!(
            decode_html(&bom, "text/html;charset=utf-8").as_deref(),
            Some("代码")
        );
        assert_eq!(
            decode_html(b"caf\xe9", "text/html;charset=iso-8859-1").as_deref(),
            Some("café")
        );
        assert_eq!(
            decode_html(b"hello", "text/html;charset=us-ascii").as_deref(),
            Some("hello")
        );
        assert_eq!(decode_html(b"hello", "text/html").as_deref(), Some("hello"));
    }

    /// 【贴图测试】【空与未知】无参数，验证空输入被拒绝，未知字符集回退 UTF-8
    #[test]
    fn decode_html_rejects_empty_and_falls_back_for_unknown_charset() {
        assert_eq!(decode_html(b"", "text/html"), None);
        assert_eq!(decode_html(b"", "text/html;charset=utf-16le"), None);
        assert_eq!(
            decode_html(b"plain", "text/html;charset=windows-1252").as_deref(),
            Some("plain")
        );
    }

    /// 【贴图测试】【RTF类型排序】无参数，优先 text/rtf 再 application/rtf，忽略其他类型并去重
    #[test]
    fn rtf_mime_candidates_prefer_text_rtf() {
        let mimes = [
            "application/rtf",
            "text/plain",
            "text/rtf;charset=utf-8",
            "text/html",
            "text/rtf",
            "application/rtf",
        ]
        .map(str::to_string);
        assert_eq!(
            rtf_mime_candidates(&mimes),
            ["text/rtf;charset=utf-8", "text/rtf", "application/rtf"]
        );
    }

    /// 【贴图测试】【RTF来源解码】无参数，UTF-8、BOM 与 Windows-1252 字节均可还原，空内容返回空值
    #[test]
    fn decode_rtf_source_accepts_utf8_bom_and_ansi() {
        assert_eq!(
            decode_rtf_source(b"{\\rtf1\\ansi hello}").as_deref(),
            Some(b"{\\rtf1\\ansi hello}".as_slice())
        );
        let mut bom = vec![0xEF, 0xBB, 0xBF];
        bom.extend("{\\rtf1 中文}".as_bytes());
        assert_eq!(
            decode_rtf_source(&bom).as_deref(),
            Some("{\\rtf1 中文}".as_bytes())
        );
        assert_eq!(
            decode_rtf_source(b"caf\xe9").as_deref(),
            Some(b"caf\xe9".as_slice())
        );
        assert_eq!(decode_rtf_source(b"  \r\n"), None);
    }
}
