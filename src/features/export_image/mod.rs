//! 【图像导出】【共享图片】只为用户保存和复制生成外框，原图交接与无界面捕获保留原始 PNG

use crate::config::export::ImageFrame;
use std::{borrow::Cow, io};

mod codec;
mod pixels;

/// 【图像导出】【按需外框】对 png 应用 frame 并返回 PNG；关闭时直接借用输入，不解码或创建图像缓冲
pub fn prepare(png: &[u8], frame: ImageFrame) -> io::Result<Cow<'_, [u8]>> {
    if !frame.enabled {
        return Ok(Cow::Borrowed(png));
    }
    let info = crate::features::capture_history::png_metadata::inspect(&mut io::Cursor::new(png))?;
    let options = frame.options();
    // 1. 【图像导出】【分配检查】在解码原图前校验物理尺寸和外框，拒绝超出图像核心上限的输入
    shot_image::export::Layout::new(info.width as i32, info.height as i32, &options)
        .map_err(io::Error::other)?;
    let source = tiny_skia::Pixmap::decode_png(png).map_err(io::Error::other)?;
    let core = pixels::to_argb(&source)?;
    drop(source);
    // 2. 【图像导出】【原版合成】在预乘空间执行原版圆角和阴影算法，内容只覆盖一次
    let output = shot_image::export::render(core.view(), &options).map_err(io::Error::other)?;
    drop(core);
    let rgba = pixels::to_rgba(&output)?;
    drop(output);
    // 3. 【图像导出】【透明输出】PNG 使用独立透明通道，阴影和裁切边缘不能以预乘颜色直接编码
    Ok(Cow::Owned(codec::encode(rgba)))
}

/// 【图像导出】【最新外框】为 png 读取当前外框配置，返回导出 PNG；读取失败按原版保留原图并记录错误
pub fn current(png: &[u8]) -> io::Result<Cow<'_, [u8]>> {
    match crate::config::export::current() {
        Ok(frame) => prepare(png, frame),
        Err(error) => {
            log::error!("【图像导出】【外框配置】读取失败，沿用原图: {error}");
            Ok(Cow::Borrowed(png))
        }
    }
}

/// 【图像导出】【输出策略】按 png 和 user_image 决定是否合成外框，返回借用原图或拥有的共享图片
pub fn for_delivery(png: &[u8], user_image: bool) -> io::Result<Cow<'_, [u8]>> {
    if user_image {
        current(png)
    } else {
        Ok(Cow::Borrowed(png))
    }
}
