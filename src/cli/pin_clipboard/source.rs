//! 【贴图】【剪贴板来源】读取原生图片、文件列表、HTML 与文本，所有缓冲都有上限

use super::{IMAGE_LIMIT, TEXT_LIMIT};
use std::path::PathBuf;

#[derive(Default)]
pub(super) struct Clipboard {
    pub image: Option<Vec<u8>>,
    pub files: Vec<PathBuf>,
    pub html: Option<String>,
    pub text: Option<String>,
    pub sequence: u32,
}

impl Clipboard {
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

/// 【贴图】【Linux类型读取】按 mime、limit 读取有界数据，缺少类型或权限时返回错误
#[cfg(target_os = "linux")]
fn data(mime: wl_clipboard_rs::paste::MimeType<'_>, limit: u64) -> crate::AppResult<Vec<u8>> {
    use std::io::Read;
    use wl_clipboard_rs::paste::{ClipboardType, MimeType, Seat, get_contents};
    if crate::backend::x11::clipboard::use_x11()? {
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
    use wl_clipboard_rs::paste::MimeType;
    let file_data = data(MimeType::Specific("text/uri-list"), TEXT_LIMIT).or_else(|_| {
        data(
            MimeType::Specific("x-special/gnome-copied-files"),
            TEXT_LIMIT,
        )
    });
    let files = file_data
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map_or_else(Vec::new, |text| self::files(&text));
    if !files.is_empty() {
        return Ok(Clipboard {
            files,
            sequence: x11_sequence().unwrap_or(0),
            ..Default::default()
        });
    }
    let image = data(MimeType::Specific("image/png"), IMAGE_LIMIT).ok();
    let html = if image.is_none() {
        data(MimeType::Specific("text/html"), TEXT_LIMIT)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    } else {
        None
    };
    let text = if image.is_none() {
        data(MimeType::Text, TEXT_LIMIT)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    } else {
        None
    };
    Ok(Clipboard {
        image,
        html,
        text,
        ..Default::default()
    })
}

/// 【贴图】【X11剪贴板版本】无参数，返回选择拥有者编号；Wayland 没有公开版本时返回空值
#[cfg(target_os = "linux")]
fn x11_sequence() -> Option<u32> {
    use x11rb::protocol::xproto::ConnectionExt;
    if !crate::backend::x11::clipboard::use_x11().ok()? {
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
    if !files.is_empty() {
        return Ok(Clipboard {
            files,
            sequence,
            ..Default::default()
        });
    }
    let image = crate::backend::clipboard::read_image_png()
        .ok()
        .filter(|bytes| bytes.len() as u64 <= IMAGE_LIMIT);
    let html = crate::backend::clipboard::read_html().ok();
    let text = crate::backend::clipboard::read_text()
        .ok()
        .filter(|text| text.len() as u64 <= TEXT_LIMIT);
    Ok(Clipboard {
        image,
        html,
        text,
        sequence,
        ..Default::default()
    })
}
