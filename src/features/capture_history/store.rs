//! 【截图历史】【存储业务】PNG 记录、查询与按数量和总字节淘汰

use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};

use super::{
    identity,
    png_metadata::{self, Prepared},
    storage::Directory,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    pub created: DateTime<Utc>,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub scale: f32,
    pub search: String,
}

#[derive(Debug, Clone)]
pub struct Store {
    directory: PathBuf,
    byte_limit: u64,
}

impl Store {
    /// 【截图历史】【访问器】用 directory 和 byte_limit 创建访问器；不创建目录或读取图像
    pub fn new(directory: impl Into<PathBuf>, byte_limit: u64) -> Self {
        Self {
            directory: directory.into(),
            byte_limit: byte_limit.max(1),
        }
    }

    /// 【截图历史】【目录位置】返回当前访问器的存储目录；不执行文件操作
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// 【截图历史】【记录查询】返回有效 PNG 记录，按新到旧排序；缺少目录返回空列表，不读取整图像素
    pub fn entries(&self) -> io::Result<Vec<Entry>> {
        let Some(directory) = Directory::open(&self.directory, false)? else {
            return Ok(Vec::new());
        };
        let mut entries = Vec::new();
        for (name, bytes) in directory.owned()? {
            if bytes > self.byte_limit.min(png_metadata::PNG_BYTE_LIMIT) {
                continue;
            }
            let info = directory
                .read(&name)
                .and_then(|mut file| png_metadata::inspect(&mut file));
            if let Ok(info) = info {
                entries.push(Entry {
                    created: identity::timestamp(&name).unwrap(),
                    name,
                    bytes,
                    width: info.width,
                    height: info.height,
                    scale: info.scale,
                    search: info.search,
                });
            }
        }
        Ok(entries)
    }

    /// 【截图历史】【完成图记录】保存 png、scale 和 search，再按 limit 与容量淘汰最旧项；返回新记录
    pub fn append(&self, png: &[u8], scale: f32, search: &str, limit: usize) -> io::Result<Entry> {
        if limit == 0 {
            return Err(io::Error::other("history limit must be positive"));
        }
        let prepared = Prepared::new(png, scale, search)?;
        if prepared.len() > self.byte_limit {
            return Err(io::Error::other("screenshot exceeds history storage limit"));
        }
        // 1. 【截图历史】【记录事务】在锁内生成递增名称，临时文件始终位于固定目录
        let directory = Directory::open(&self.directory, true)?
            .ok_or_else(|| io::Error::other("cannot create history directory"))?;
        let _lock = directory.lock()?;
        let previous = directory.owned()?;
        let name = identity::next(
            Utc::now(),
            previous
                .first()
                .and_then(|entry| identity::timestamp(&entry.0)),
        )?;
        let mut file = tempfile::NamedTempFile::new_in(directory.path())?;
        prepared.write(&mut file)?;
        file.as_file().sync_all()?;
        file.persist_noclobber(directory.path().join(&name))
            .map_err(|error| error.error)?;
        directory.sync()?;
        // 2. 【截图历史】【容量淘汰】新记录成功落盘后才删除旧项，超大截图不会破坏已有历史
        retain(&directory, limit, self.byte_limit)?;
        directory.sync()?;
        Ok(Entry {
            created: identity::timestamp(&name).unwrap(),
            name,
            bytes: prepared.len(),
            width: prepared.info.width,
            height: prepared.info.height,
            scale: prepared.info.scale,
            search: prepared.info.search,
        })
    }

    /// 【截图历史】【原图打开】返回 name 对应的只读普通文件；拒绝非法身份、链接及过大文件
    pub fn open(&self, name: &str) -> io::Result<File> {
        let directory = Directory::open(&self.directory, false)?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "history directory does not exist")
        })?;
        let file = directory.read(name)?;
        if file.metadata()?.len() > self.byte_limit.min(png_metadata::PNG_BYTE_LIMIT) {
            return Err(io::Error::other("history file is too large"));
        }
        Ok(file)
    }

    /// 【截图历史】【原图读取】返回 name 的 PNG 字节和有效设备比例；有界读取并拒绝损坏结构
    pub fn read(&self, name: &str) -> io::Result<(Vec<u8>, f32)> {
        let mut file = self.open(name)?;
        let info = png_metadata::inspect(&mut file)?;
        use std::io::{Seek, SeekFrom};
        file.seek(SeekFrom::Start(0))?;
        png_metadata::verify(std::io::BufReader::new(&mut file))?;
        file.seek(SeekFrom::Start(0))?;
        let mut png = Vec::new();
        let limit = self.byte_limit.min(png_metadata::PNG_BYTE_LIMIT);
        file.take(limit + 1).read_to_end(&mut png)?;
        if png.len() as u64 > limit {
            return Err(io::Error::other("history file grew beyond its limit"));
        }
        Ok((png, info.scale))
    }

    /// 【截图历史】【删除记录】在锁内删除 name；不存在视为成功，非法身份和链接返回错误
    pub fn remove(&self, name: &str) -> io::Result<()> {
        if identity::timestamp(name).is_none() {
            return Err(io::Error::other("invalid history identity"));
        }
        let Some(directory) = Directory::open(&self.directory, false)? else {
            return Ok(());
        };
        let _lock = directory.lock()?;
        directory.remove(name)?;
        directory.sync()
    }

    /// 【截图历史】【策略清理】按 limit 和当前字节上限删除最旧项；目录不存在时不创建目录
    pub fn prune(&self, limit: usize) -> io::Result<()> {
        if limit == 0 {
            return Err(io::Error::other("history limit must be positive"));
        }
        let Some(directory) = Directory::open(&self.directory, false)? else {
            return Ok(());
        };
        let _lock = directory.lock()?;
        retain(&directory, limit, self.byte_limit)?;
        directory.sync()
    }

    /// 【截图历史】【清空记录】在锁内删除全部合法普通历史文件，包括损坏 PNG；保留其他文件和链接
    pub fn clear(&self) -> io::Result<()> {
        let Some(directory) = Directory::open(&self.directory, false)? else {
            return Ok(());
        };
        let _lock = directory.lock()?;
        for (name, _) in directory.owned()? {
            directory.remove(&name)?;
        }
        directory.sync()
    }
}

/// 【截图历史】【保留前缀】保留 directory 中最新 limit 项并满足 byte_limit；调用方必须已经持锁
fn retain(directory: &Directory, limit: usize, byte_limit: u64) -> io::Result<()> {
    let mut total = 0_u64;
    for (index, (name, bytes)) in directory.owned()?.iter().enumerate() {
        total = total.saturating_add(*bytes);
        if index >= limit.min(200) || total > byte_limit {
            directory.remove(name)?;
        }
    }
    Ok(())
}
