//! 【截图历史】【PNG 信息】有界读取文本信息，直接插入元数据而不改变图像数据块

use std::{
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    ops::Range,
};

pub const DPR_KEY: &str = "mark-shot-device-pixel-ratio";
pub const SEARCH_KEY: &str = "mark-shot-history-search";
pub const SEARCH_LIMIT: usize = 4096;
pub const PIXEL_LIMIT: u64 = 64 * 1024 * 1024;
pub const PNG_BYTE_LIMIT: u64 = 320 * 1024 * 1024;
const SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const META_LIMIT: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct PngInfo {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub search: String,
}

/// 【截图历史】【格式错误】返回 PNG 结构错误，message 为具体原因
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// 【截图历史】【块校验】计算 kind 与 data 的 CRC；返回校验值，不处理像素内容
fn crc(kind: &[u8], data: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(kind);
    hasher.update(data);
    hasher.finalize()
}

/// 【截图历史】【搜索限制】规范 text 的控制字符并限制 UTF-8 字节长度；返回安全搜索文本
pub fn bounded_search(text: &str) -> String {
    let mut result = String::new();
    for ch in text
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
    {
        if result.len() + ch.len_utf8() > SEARCH_LIMIT {
            break;
        }
        result.push(ch);
    }
    result
}

/// 【截图历史】【标注汇总】将 texts 合并为有界搜索文字；不分配全部标注正文，也不启动 OCR
pub fn joined_search<'a>(texts: impl IntoIterator<Item = &'a str>) -> String {
    let mut result = String::new();
    for text in texts {
        if !result.is_empty() && result.len() < SEARCH_LIMIT {
            result.push(' ');
        }
        for ch in text
            .chars()
            .map(|ch| if ch.is_control() { ' ' } else { ch })
        {
            if result.len() + ch.len_utf8() > SEARCH_LIMIT {
                return result;
            }
            result.push(ch);
        }
    }
    result
}

/// 【截图历史】【原图验证】验证 reader 的 PNG 数据流，不分配整图像素；返回解码或校验错误
pub(crate) fn verify(reader: impl std::io::BufRead + Seek) -> io::Result<()> {
    let mut decoder = png::Decoder::new_with_limits(
        reader,
        png::Limits {
            bytes: 4 * 1024 * 1024,
        },
    );
    decoder.set_ignore_text_chunk(true);
    let mut reader = decoder.read_info().map_err(io::Error::other)?;
    while reader.next_row().map_err(io::Error::other)?.is_some() {}
    reader.finish().map_err(io::Error::other)
}

/// 【截图历史】【文本拆分】从 kind、data 解析未压缩 PNG 文本；不解压外部文本，返回键和值
fn text<'a>(kind: &[u8], data: &'a [u8]) -> Option<(&'a [u8], &'a [u8])> {
    let end = data.iter().position(|byte| *byte == 0)?;
    let key = &data[..end];
    let mut value = &data[end + 1..];
    match kind {
        b"tEXt" => {}
        b"iTXt" => {
            if value.get(..2)? != [0, 0] {
                return None;
            }
            value = &value[2..];
            for _ in 0..2 {
                value = &value[value.iter().position(|byte| *byte == 0)? + 1..];
            }
        }
        _ => return None,
    }
    Some((key, value))
}

/// 【截图历史】【文本识别】判断 kind、data 是否为历史拥有的文本块；包含压缩旧块也可移除
fn owned_text(kind: &[u8], data: &[u8]) -> bool {
    matches!(kind, b"tEXt" | b"iTXt" | b"zTXt") && data.iter().position(|byte| *byte == 0)
        .is_some_and(|end| matches!(&data[..end], key if key == DPR_KEY.as_bytes() || key == SEARCH_KEY.as_bytes()))
}

/// 【截图历史】【头部校验】检查 IHDR data，返回尺寸；拒绝非法颜色、空图和超出像素上限的声明
fn header(data: &[u8]) -> io::Result<(u32, u32)> {
    if data.len() != 13 {
        return Err(invalid("invalid PNG header length"));
    }
    let width = u32::from_be_bytes(data[..4].try_into().unwrap());
    let height = u32::from_be_bytes(data[4..8].try_into().unwrap());
    let depth = data[8];
    let valid_color = match data[9] {
        0 => matches!(depth, 1 | 2 | 4 | 8 | 16),
        2 | 4 | 6 => matches!(depth, 8 | 16),
        3 => matches!(depth, 1 | 2 | 4 | 8),
        _ => false,
    };
    if width == 0
        || height == 0
        || u64::from(width) * u64::from(height) > PIXEL_LIMIT
        || width > 65_536
        || !valid_color
        || data[10] != 0
        || data[11] != 0
        || data[12] > 1
    {
        return Err(invalid("invalid or oversized PNG dimensions"));
    }
    Ok((width, height))
}

/// 【截图历史】【信息读取】检查 reader 的 PNG 结构并按需读取文本，跳过图像数据；返回尺寸、比例和搜索信息
pub fn inspect(reader: &mut (impl Read + Seek)) -> io::Result<PngInfo> {
    let length = reader.seek(SeekFrom::End(0))?;
    if length > PNG_BYTE_LIMIT {
        return Err(invalid("PNG exceeds the per-image byte limit"));
    }
    reader.seek(SeekFrom::Start(0))?;
    let mut signature = [0; 8];
    reader.read_exact(&mut signature)?;
    if &signature != SIGNATURE {
        return Err(invalid("not a PNG file"));
    }
    let mut info = None;
    let mut image_data = false;
    let mut metadata_bytes = 0;
    let mut offset = 8_u64;
    for _ in 0..65_536 {
        let mut chunk = [0; 8];
        reader.read_exact(&mut chunk)?;
        let count = u32::from_be_bytes(chunk[..4].try_into().unwrap()) as usize;
        let kind = &chunk[4..];
        let end = offset
            .checked_add(count as u64 + 12)
            .filter(|end| *end <= length)
            .ok_or_else(|| invalid("truncated PNG chunk"))?;
        if info.is_none() && kind != b"IHDR" {
            return Err(invalid("PNG header is missing"));
        }
        if kind == b"IHDR"
            || (matches!(kind, b"tEXt" | b"iTXt")
                && count <= META_LIMIT
                && metadata_bytes + count <= META_LIMIT)
        {
            if kind == b"IHDR" && (info.is_some() || count != 13) {
                return Err(invalid("invalid PNG header"));
            }
            let mut data = vec![0; count];
            reader.read_exact(&mut data)?;
            let mut checksum = [0; 4];
            reader.read_exact(&mut checksum)?;
            let valid_crc = crc(kind, &data) == u32::from_be_bytes(checksum);
            if kind == b"IHDR" {
                if !valid_crc {
                    return Err(invalid("PNG header checksum mismatch"));
                }
                let (width, height) = header(&data)?;
                info = Some(PngInfo {
                    width,
                    height,
                    scale: 1.0,
                    search: String::new(),
                });
            } else {
                metadata_bytes += count;
                if valid_crc && let Some((key, value)) = text(kind, &data) {
                    let current = info.as_mut().unwrap();
                    if key == DPR_KEY.as_bytes() {
                        current.scale = std::str::from_utf8(value)
                            .ok()
                            .and_then(|value| value.parse::<f32>().ok())
                            .filter(|scale| (1.0..=16.0).contains(scale))
                            .unwrap_or(1.0);
                    } else if key == SEARCH_KEY.as_bytes() {
                        current.search = std::str::from_utf8(value)
                            .map(bounded_search)
                            .unwrap_or_default();
                    }
                }
            }
        } else {
            reader.seek(SeekFrom::Start(end))?;
        }
        image_data |= kind == b"IDAT" && count > 0;
        if kind == b"IEND" {
            if count != 0 || !image_data || end != length {
                return Err(invalid("invalid PNG end"));
            }
            reader.seek(SeekFrom::Start(end - 4))?;
            let mut checksum = [0; 4];
            reader.read_exact(&mut checksum)?;
            if u32::from_be_bytes(checksum) != crc(b"IEND", &[]) {
                return Err(invalid("PNG end checksum mismatch"));
            }
            return Ok(info.unwrap());
        }
        offset = end;
    }
    Err(invalid("too many PNG chunks"))
}

pub(super) struct Prepared<'a> {
    input: &'a [u8],
    chunks: Vec<Range<usize>>,
    metadata: Vec<u8>,
    pub info: PngInfo,
}

impl<'a> Prepared<'a> {
    /// 【截图历史】【保存准备】校验 png 并准备比例与 search 文本；返回原始数据切片，不分配第二份整图
    pub fn new(png: &'a [u8], scale: f32, search: &str) -> io::Result<Self> {
        let mut info = inspect(&mut Cursor::new(png))?;
        info.scale = if (1.0..=16.0).contains(&scale) {
            scale
        } else {
            1.0
        };
        info.search = bounded_search(search);
        let mut metadata = Vec::new();
        chunk(
            &mut metadata,
            b"tEXt",
            format!("{DPR_KEY}\0{}", info.scale).as_bytes(),
        );
        if !info.search.is_empty() {
            chunk(
                &mut metadata,
                b"iTXt",
                format!("{SEARCH_KEY}\0\0\0\0\0{}", info.search).as_bytes(),
            );
        }
        let mut chunks = Vec::new();
        let mut offset = 8;
        while offset < png.len() {
            let count = u32::from_be_bytes(png[offset..offset + 4].try_into().unwrap()) as usize;
            let end = offset + count + 12;
            let kind = &png[offset + 4..offset + 8];
            let data = &png[offset + 8..end - 4];
            if crc(kind, data) != u32::from_be_bytes(png[end - 4..end].try_into().unwrap()) {
                return Err(invalid("PNG checksum mismatch"));
            }
            if !owned_text(kind, data) {
                chunks.push(offset..end);
            }
            offset = end;
        }
        Ok(Self {
            input: png,
            chunks,
            metadata,
            info,
        })
    }

    /// 【截图历史】【编码大小】返回插入元数据后的完整 PNG 字节数；不复制像素
    pub fn len(&self) -> u64 {
        (8 + self.metadata.len() + self.chunks.iter().map(|chunk| chunk.len()).sum::<usize>())
            as u64
    }

    /// 【截图历史】【无损写入】将原始 PNG 数据块与新元数据写入 writer；像素、调色板和 alpha 保持不变
    pub fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        writer.write_all(SIGNATURE)?;
        for (index, range) in self.chunks.iter().enumerate() {
            writer.write_all(&self.input[range.clone()])?;
            if index == 0 {
                writer.write_all(&self.metadata)?;
            }
        }
        Ok(())
    }
}

/// 【截图历史】【文本块编码】把 kind、data 及校验值追加到 output；只用于少量元数据
fn chunk(output: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(kind);
    output.extend_from_slice(data);
    output.extend_from_slice(&crc(kind, data).to_be_bytes());
}
