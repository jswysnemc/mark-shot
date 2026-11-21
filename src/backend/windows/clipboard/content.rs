//! 【Windows剪贴板】【文件及HTML】有界读取复制文件和原生 HTML 格式

use super::{Opened, memory};
use std::path::PathBuf;
use windows_sys::Win32::{
    System::DataExchange::{GetClipboardData, RegisterClipboardFormatW},
    UI::Shell::DragQueryFileW,
};

/// 【Windows剪贴板】【复制文件】无参数，通过 CF_HDROP 读取至多二百五十六个 Unicode 文件路径，返回本地路径列表
pub fn read_files() -> crate::AppResult<Vec<PathBuf>> {
    // 1. 【Windows剪贴板】【文件校验】确认原生格式及有界内存，拒绝超量文件
    let _clipboard = Opened::read()?;
    let bytes = memory::read(15, 1024 * 1024)?;
    if bytes.len() < 20 {
        return Err("invalid clipboard file list".into());
    }
    let handle = unsafe { GetClipboardData(15) };
    let count = unsafe { DragQueryFileW(handle.cast(), u32::MAX, std::ptr::null_mut(), 0) };
    if count > 256 {
        return Err("clipboard contains too many files".into());
    }
    let mut files = Vec::new();
    // 2. 【Windows剪贴板】【路径读取】逐项读取完整 Unicode 路径，确认长度未发生变化
    for index in 0..count {
        let size = unsafe { DragQueryFileW(handle.cast(), index, std::ptr::null_mut(), 0) };
        if size == 0 || size > 32768 {
            return Err("invalid clipboard path length".into());
        }
        let mut path = vec![0_u16; size as usize + 1];
        let written = unsafe { DragQueryFileW(handle.cast(), index, path.as_mut_ptr(), size + 1) };
        if written != size {
            return Err("clipboard file list changed".into());
        }
        files.push(PathBuf::from(String::from_utf16(&path[..size as usize])?));
    }
    Ok(files)
}

/// 【Windows剪贴板】【标记上限】富文本 HTML/RTF 的字节上限，避免逐字符样式导致正文提前截断
const MARKUP_LIMIT: usize = 16 * 1024 * 1024;

/// 【Windows剪贴板】【HTML读取】无参数，先读 HTML Format 再退回 text/html，返回最多十六 MiB 的 UTF-8 HTML 正文
pub fn read_html() -> crate::AppResult<String> {
    let _clipboard = Opened::read()?;
    let bytes =
        payload("HTML Format", MARKUP_LIMIT).or_else(|_| payload("text/html", MARKUP_LIMIT))?;
    Ok(decode_payload(&bytes))
}

/// 【Windows剪贴板】【RTF读取】无参数，先读 Rich Text Format 再退回 text/rtf，返回最多十六 MiB 的 RTF 正文
pub fn read_rtf() -> crate::AppResult<Vec<u8>> {
    let _clipboard = Opened::read()?;
    let bytes =
        payload("Rich Text Format", MARKUP_LIMIT).or_else(|_| payload("text/rtf", MARKUP_LIMIT))?;
    Ok(bytes)
}

/// 【Windows剪贴板】【格式数据】按 name 与 limit 登记格式并读取有界字节，登记或读取失败返回错误
fn payload(name: &str, limit: usize) -> crate::AppResult<Vec<u8>> {
    let name: Vec<_> = name.encode_utf16().chain([0]).collect();
    let format = unsafe { RegisterClipboardFormatW(name.as_ptr()) };
    if format == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(memory::read(format, limit)?)
}

/// 【Windows剪贴板】【负载解码】按 BOM、零字节分布或 UTF-8 解码 HTML 负载，容忍 UTF-16 与末尾终止符
fn decode_payload(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return decode_utf16(rest, true);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return decode_utf16(rest, false);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return decode_utf8(rest);
    }
    let nul_count = bytes.iter().filter(|byte| **byte == 0).count();
    if bytes.len() >= 4 && nul_count * 4 >= bytes.len() {
        let even = bytes.iter().step_by(2).filter(|byte| **byte == 0).count();
        let odd = bytes
            .iter()
            .skip(1)
            .step_by(2)
            .filter(|byte| **byte == 0)
            .count();
        return decode_utf16(bytes, odd >= even);
    }
    decode_utf8(bytes)
}

/// 【Windows剪贴板】【UTF8负载】按字节在首个零字节处截断并严格解码，非法序列使用有损回退，移除 BOM
fn decode_utf8(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    let text = std::str::from_utf8(&bytes[..end]).map_or_else(
        |_| String::from_utf8_lossy(&bytes[..end]).into_owned(),
        str::to_owned,
    );
    text.strip_prefix('\u{feff}').unwrap_or(&text).to_string()
}

/// 【Windows剪贴板】【UTF16负载】按 little_endian 解码双字节单元，忽略末尾奇数字节并在首个零单元处停止
fn decode_utf16(bytes: &[u8], little_endian: bool) -> String {
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
    let text = String::from_utf16_lossy(&units);
    text.strip_prefix('\u{feff}').unwrap_or(&text).to_string()
}

#[cfg(test)]
mod tests {
    use super::decode_payload;

    /// 【Windows剪贴板测试】【UTF16LE编码】按 text 返回小端 UTF-16 字节，测试用
    fn utf16le(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    /// 【Windows剪贴板测试】【UTF16BE编码】按 text 返回大端 UTF-16 字节，测试用
    fn utf16be(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_be_bytes).collect()
    }

    /// 【Windows剪贴板测试】【负载编码】无参数，验证 UTF-8、BOM、UTF-16 与零终止负载的解码
    #[test]
    fn payload_decoding_accepts_utf8_and_utf16() {
        assert_eq!(decode_payload(b"<p>a</p>\0"), "<p>a</p>");
        assert_eq!(decode_payload("\u{feff}<p>中</p>".as_bytes()), "<p>中</p>");
        let mut little = vec![0xFF, 0xFE];
        little.extend(utf16le("<p>代码</p>"));
        little.extend([0, 0]);
        assert_eq!(decode_payload(&little), "<p>代码</p>");
        let mut big = vec![0xFE, 0xFF];
        big.extend(utf16be("<p>代码</p>"));
        assert_eq!(decode_payload(&big), "<p>代码</p>");
        let mut bare = utf16le("<p>ab</p>");
        bare.extend([0, 0]);
        assert_eq!(decode_payload(&bare), "<p>ab</p>");
    }
}
