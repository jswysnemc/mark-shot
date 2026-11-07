//! 【Windows剪贴板】【系统拥有】图片同时提供PNG与DIBV5，文本使用Unicode，进程退出后系统继续持有数据

mod image;
#[cfg(test)]
mod image_tests;
mod memory;
#[cfg(test)]
mod system_tests;
use memory::{Buffer, Opened};
use windows_sys::Win32::System::DataExchange::RegisterClipboardFormatW;

pub struct Provider;
pub const DAEMON_ARG: &str = "--clipboard-daemon";
pub const TEXT_ARG: &str = "text";
pub const PRIMARY_ARG: &str = "primary";
pub const READY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
const UNICODE: u32 = 13;

/// 【Windows剪贴板】【PNG类型】无参数，返回系统登记的PNG格式编号，登记失败返回系统错误
fn png_format() -> std::io::Result<u32> {
    let value = unsafe { RegisterClipboardFormatW([80, 78, 71, 0].as_ptr()) };
    if value == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(value)
    }
}

impl crate::backend::ClipboardProvider for Provider {
    /// 【Windows剪贴板】【原图复制】按png_data提供PNG及透明DIBV5，返回所有权交接结果，不启动常驻进程
    fn copy_image_to_clipboard(&self, png_data: &[u8]) -> crate::AppResult {
        let dib = image::dib(png_data)?;
        let png = Buffer::new(png_data)?;
        let dib = Buffer::new(&dib)?;
        let clipboard = Opened::write()?;
        clipboard.clear()?;
        png.give(png_format()?)?;
        dib.give(17)?;
        Ok(())
    }
}

/// 【Windows剪贴板】【文字复制】按text提供以零终止的UTF16文本，返回系统交接结果，正文不经过本地编码转换
pub fn copy_text(text: &str) -> crate::AppResult {
    if text.len() > 16 * 1024 * 1024 || text.contains('\0') {
        return Err("invalid clipboard text".into());
    }
    let bytes = text
        .encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    let memory = Buffer::new(&bytes)?;
    let clipboard = Opened::write()?;
    clipboard.clear()?;
    memory.give(UNICODE)?;
    Ok(())
}

/// 【Windows剪贴板】【文字接收】无参数，返回有界Unicode正文，格式不可用或无效时返回错误
pub fn read_text() -> crate::AppResult<String> {
    let _clipboard = Opened::read()?;
    let bytes = memory::read(UNICODE, 16 * 1024 * 1024)?;
    let mut units = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|unit| u16::from_le_bytes(*unit))
        .take_while(|unit| *unit != 0);
    Ok(String::from_utf16(&units.by_ref().collect::<Vec<_>>())?)
}

/// 【Windows剪贴板】【图像接收】无参数，优先接收原PNG，否则转换DIB，返回有界完整PNG或格式错误
pub fn read_image_png() -> crate::AppResult<Vec<u8>> {
    let _clipboard = Opened::read()?;
    if let Ok(png) = memory::read(png_format()?, 128 * 1024 * 1024) {
        return Ok(png);
    }
    let dib =
        memory::read(17, 128 * 1024 * 1024).or_else(|_| memory::read(8, 128 * 1024 * 1024))?;
    image::png(&dib)
}

pub mod daemon {
    /// 【Windows剪贴板】【辅助兼容】按args读取有界标准输入并一次性交接，首行确认后退出，不保持系统数据来源进程
    pub fn cli(args: impl Iterator<Item = String>) {
        use crate::backend::ClipboardProvider;
        use std::io::{Read, Write};
        let text = args.into_iter().any(|arg| arg == super::TEXT_ARG);
        let mut bytes = Vec::new();
        let result: crate::AppResult = (|| {
            std::io::stdin()
                .take(128 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > 128 * 1024 * 1024 {
                return Err("clipboard data exceeds limit".into());
            }
            if text {
                super::copy_text(std::str::from_utf8(&bytes)?)
            } else {
                super::Provider.copy_image_to_clipboard(&bytes)
            }
        })();
        match result {
            Ok(()) => {
                println!();
                let _ = std::io::stdout().flush();
            }
            Err(error) => {
                println!("{error}");
                std::process::exit(1);
            }
        }
    }
}
