//! 【图片剪贴板】【文件来源】有界接收和发送原始 PNG，避免服务进程保留完整图片堆缓冲

use std::{
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, OwnedFd},
        unix::fs::FileExt,
    },
    path::Path,
    sync::OnceLock,
};

const CHUNK: usize = 64 * 1024;
const RELEASE_BATCH: u64 = 4 * 1024 * 1024;

pub(super) struct ImageSource {
    file: File,
    pub(super) bytes: u64,
    bitmap: OnceLock<Result<File, String>>,
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
        Ok(Self {
            file,
            bytes,
            bitmap: OnceLock::new(),
        })
    }

    /// 【图片剪贴板】【数据发送】按独立文件偏移把完整 PNG 写入 output，返回传输结果，多次粘贴不共享读取位置
    pub(super) fn send(&self, mut output: impl Write) -> io::Result<()> {
        Self::send_file(&self.file, self.bytes, &mut output)
    }

    /// 【图片剪贴板】【文件传输】按 file 与 bytes 从独立偏移完整发送到 output，返回传输结果
    fn send_file(file: &File, bytes: u64, mut output: impl Write) -> io::Result<()> {
        let mut buffer = [0; CHUNK];
        let mut offset = 0;
        let mut released = 0;
        while offset < bytes {
            let count = file.read_at(&mut buffer, offset)?;
            if count == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "clipboard image truncated",
                ));
            }
            output.write_all(&buffer[..count])?;
            offset += count as u64;
            if offset - released >= RELEASE_BATCH {
                release_pages(file, released, offset - released)?;
                released = offset;
            }
        }
        release_pages(file, 0, bytes)?;
        Ok(())
    }

    /// 【图片剪贴板】【发送到 fd】清除 fd 的非阻塞标志后按 mime 完整发送对应图像，返回传输结果
    pub(super) fn send_fd(&self, fd: impl Into<OwnedFd>, mime: &str) -> io::Result<()> {
        // 1. 【图片剪贴板】【管道配置】发送线程清除接收端传入的非阻塞标志，避免写满管道时中止图片
        let file = File::from(fd.into());
        let flags = nix::fcntl::OFlag::from_bits_truncate(nix::fcntl::fcntl(
            &file,
            nix::fcntl::FcntlArg::F_GETFL,
        )?);
        nix::fcntl::fcntl(
            &file,
            nix::fcntl::FcntlArg::F_SETFL(flags & !nix::fcntl::OFlag::O_NONBLOCK),
        )?;
        match mime {
            super::image_formats::PNG => self.send(&file),
            super::image_formats::BMP | super::image_formats::X_BMP => {
                // 2. 【图片剪贴板】【按需缓存】并发请求只编码一次，驻留服务持有匿名文件而非完整位图堆缓冲
                let bitmap = self.bitmap.get_or_init(|| {
                    super::image_formats::bitmap(&self.file).map_err(|error| error.to_string())
                });
                let bitmap = bitmap
                    .as_ref()
                    .map_err(|error| io::Error::other(error.clone()))?;
                Self::send_file(bitmap, bitmap.metadata()?.len(), &file)
            }
            _ => Err(io::Error::other("unsupported clipboard image format")),
        }
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

    /// 【图片剪贴板测试】【非阻塞管道】接收端交来的 fd 带 O_NONBLOCK 时仍完整发送；无返回值
    #[test]
    fn file_source_completes_transfer_over_nonblocking_fd() {
        let directory = tempfile::tempdir().unwrap();
        let expected: Vec<u8> = (0..RELEASE_BATCH as usize + 913)
            .map(|index| (index * 53) as u8)
            .collect();
        let source = ImageSource::receive(expected.as_slice(), directory.path()).unwrap();

        // 1. 【图片剪贴板测试】【接收端管道】模拟带非阻塞标志的真实传输描述符
        let (read_end, write_end) = nix::unistd::pipe().unwrap();
        let flags = nix::fcntl::OFlag::from_bits_truncate(
            nix::fcntl::fcntl(&write_end, nix::fcntl::FcntlArg::F_GETFL).unwrap(),
        );
        nix::fcntl::fcntl(
            &write_end,
            nix::fcntl::FcntlArg::F_SETFL(flags | nix::fcntl::OFlag::O_NONBLOCK),
        )
        .unwrap();

        let reader = std::thread::spawn(move || {
            let mut file = File::from(read_end);
            let mut received = Vec::new();
            file.read_to_end(&mut received).unwrap();
            received
        });

        source
            .send_fd(write_end, super::super::image_formats::PNG)
            .unwrap();
        assert_eq!(reader.join().unwrap(), expected);
    }
}
