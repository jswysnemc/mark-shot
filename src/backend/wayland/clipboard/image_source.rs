//! 【图片剪贴板】【文件来源】有界接收和发送原始 PNG，避免服务进程保留完整图片堆缓冲

use std::{
    fs::File,
    io::{self, Read, Write},
    os::{fd::AsRawFd, unix::fs::FileExt},
    path::Path,
};

const CHUNK: usize = 64 * 1024;
const RELEASE_BATCH: u64 = 4 * 1024 * 1024;

pub(super) struct ImageSource {
    file: File,
    pub(super) bytes: u64,
}

impl ImageSource {
    /// 【图片剪贴板】【接收数据】从 input 分块接收 PNG 到 root 内匿名文件，返回唯一文件来源或 I/O 错误
    pub(super) fn receive(mut input: impl Read, root: &Path) -> io::Result<Self> {
        std::fs::create_dir_all(root)?;
        let mut file = tempfile::tempfile_in(root)?;
        let mut buffer = [0; CHUNK];
        let (mut bytes, mut released) = (0, 0);
        loop {
            let count = match input.read(&mut buffer) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                result => result?,
            };
            if count == 0 {
                break;
            }
            file.write_all(&buffer[..count])?;
            bytes += count as u64;
            if bytes - released >= RELEASE_BATCH {
                file.sync_data()?;
                release_pages(&file, released, bytes - released)?;
                released = bytes;
            }
        }
        file.sync_data()?;
        release_pages(&file, released, bytes - released)?;
        advise(&file, 0, 0, nix::libc::POSIX_FADV_RANDOM)?;
        Ok(Self { file, bytes })
    }

    /// 【图片剪贴板】【数据发送】按独立文件偏移把完整 PNG 写入 output，返回传输结果，多次粘贴不共享读取位置
    pub(super) fn send(&self, mut output: impl Write) -> io::Result<()> {
        let mut buffer = [0; CHUNK];
        let mut offset = 0;
        let mut released = 0;
        while offset < self.bytes {
            let count = self.file.read_at(&mut buffer, offset)?;
            if count == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "clipboard image truncated",
                ));
            }
            output.write_all(&buffer[..count])?;
            offset += count as u64;
            if offset - released >= RELEASE_BATCH {
                release_pages(&self.file, released, offset - released)?;
                released = offset;
            }
        }
        release_pages(&self.file, 0, self.bytes)?;
        Ok(())
    }
}

/// 【图片剪贴板】【缓存回收】将 file 已完成的 offset、bytes 范围交还文件缓存，返回系统结果，不删除图片内容
fn release_pages(file: &File, offset: u64, bytes: u64) -> io::Result<()> {
    if bytes == 0 {
        return Ok(());
    }
    advise(file, offset, bytes, nix::libc::POSIX_FADV_DONTNEED)
}

/// 【图片剪贴板】【访问策略】对 file 的 offset、bytes 设置 advice 文件提示，返回内核错误，零长度表示文件尾
fn advise(file: &File, offset: u64, bytes: u64, advice: nix::libc::c_int) -> io::Result<()> {
    let result =
        unsafe { nix::libc::posix_fadvise(file.as_raw_fd(), offset as i64, bytes as i64, advice) };
    if result != 0 {
        return Err(io::Error::from_raw_os_error(result));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【图片剪贴板测试】【重复粘贴】无参数，检查跨批次接收及并发粘贴逐字节一致；无返回值
    #[test]
    fn file_source_serves_independent_complete_transfers() {
        let directory = tempfile::tempdir().unwrap();
        let expected: Vec<u8> = (0..RELEASE_BATCH as usize + 913)
            .map(|index| (index * 53) as u8)
            .collect();
        let source = ImageSource::receive(expected.as_slice(), directory.path()).unwrap();
        assert_eq!(source.bytes, expected.len() as u64);
        std::thread::scope(|scope| {
            for _ in 0..3 {
                scope.spawn(|| {
                    let mut output = Vec::new();
                    source.send(&mut output).unwrap();
                    assert_eq!(output, expected);
                });
            }
        });
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}
