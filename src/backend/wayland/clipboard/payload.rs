//! 【Wayland剪贴板】【数据格式】共享原始图像或文字，按需缓存兼容位图

use super::{context::Content, image_formats};
use std::{
    io::{Read, Write},
    sync::{Arc, OnceLock},
};

pub(super) struct Payload {
    bytes: Arc<[u8]>,
    image: bool,
    bitmap: OnceLock<Result<Arc<[u8]>, String>>,
}

impl Payload {
    /// 【Wayland剪贴板】【内容创建】接收 content 并返回唯一原始字节来源，图像兼容格式延迟生成
    pub fn new(content: Content) -> Self {
        let (bytes, image) = match content {
            Content::Image(bytes) => (bytes.into(), true),
            Content::Text(text) => (text.into_bytes().into(), false),
        };
        Self {
            bytes,
            image,
            bitmap: OnceLock::new(),
        }
    }

    /// 【Wayland剪贴板】【类型列表】无参数，返回图片或 UTF-8 文字支持的类型列表
    pub fn mimes(&self) -> &[&str] {
        if self.image {
            &[
                image_formats::PNG,
                image_formats::BMP,
                image_formats::X_BMP,
                "x-kde-force-image-copy",
            ]
        } else {
            &["text/plain;charset=utf-8", "text/plain", "UTF8_STRING"]
        }
    }

    /// 【Wayland剪贴板】【类型数据】按 mime 返回共享字节；未知类型返回空值，位图编码错误返回明确错误
    pub fn bytes(&self, mime: &str) -> crate::AppResult<Option<Arc<[u8]>>> {
        if !self.mimes().contains(&mime) {
            return Ok(None);
        }
        if mime == "x-kde-force-image-copy" {
            return Ok(Some(Arc::from([])));
        }
        if self.image && matches!(mime, image_formats::BMP | image_formats::X_BMP) {
            let bitmap = self.bitmap.get_or_init(|| {
                let mut source = tempfile::tempfile().map_err(|error| error.to_string())?;
                source
                    .write_all(&self.bytes)
                    .map_err(|error| error.to_string())?;
                let mut bitmap =
                    image_formats::bitmap(&source).map_err(|error| error.to_string())?;
                use std::io::{Seek, SeekFrom};
                bitmap
                    .seek(SeekFrom::Start(0))
                    .map_err(|error| error.to_string())?;
                let mut bytes = Vec::new();
                bitmap
                    .read_to_end(&mut bytes)
                    .map_err(|error| error.to_string())?;
                Ok(bytes.into())
            });
            return bitmap.clone().map(Some).map_err(Into::into);
        }
        Ok(Some(self.bytes.clone()))
    }
}
