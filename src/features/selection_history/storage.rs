//! 【历史选区】【本地存储】小型坐标文件有界读取并原子替换，不保存图像内容

use super::{Entry, LIMIT};
use std::{
    io::{self, Read, Write},
    path::Path,
};

/// 【历史选区】【有界读取】按 path 返回最多 LIMIT 条有效坐标，缺失文件返回空列表，超限或格式损坏返回错误
pub(super) fn read(path: &Path) -> io::Result<Vec<Entry>> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    file.take(65_537).read_to_end(&mut bytes)?;
    if bytes.len() > 65_536 {
        return Err(io::Error::other("selection history is too large"));
    }
    let mut entries: Vec<Entry> = serde_json::from_slice(&bytes)?;
    entries.retain(|entry| {
        let [x, y, w, h] = entry.rect;
        tiny_skia::Rect::from_xywh(x, y, w, h).is_some()
            && w >= 8.0
            && h >= 8.0
            && entry.displays.len() <= 32
            && !entry.displays.is_empty()
            && entry
                .displays
                .iter()
                .all(|(_, _, w, h, scale)| *w > 0 && *h > 0 && scale.is_finite() && *scale > 0.0)
    });
    if entries.len() > LIMIT {
        entries.drain(..entries.len() - LIMIT);
    }
    Ok(entries)
}

/// 【历史选区】【原子写入】按 path 将 entries 写入同目录临时文件后替换，返回文件系统错误
fn write(path: &Path, entries: &[Entry]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("selection history directory is unavailable"))?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(&serde_json::to_vec(entries)?)?;
    file.flush()?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}

/// 【历史选区】【并发合并】按 path 加锁重读并合并 entry 后原子保存，返回最新有限历史，避免多个会话丢失彼此新增记录
pub(super) fn record(path: &Path, entry: &Entry) -> io::Result<Vec<Entry>> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("selection history directory is unavailable"))?;
    std::fs::create_dir_all(parent)?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path.with_extension("lock"))?;
    lock.lock()?;
    let mut entries = read(path)?;
    entries.retain(|previous| previous != entry);
    entries.push(entry.clone());
    if entries.len() > LIMIT {
        entries.drain(..entries.len() - LIMIT);
    }
    write(path, &entries)?;
    Ok(entries)
}
