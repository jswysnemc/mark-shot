//! 【贴图】【重复文件状态】短事务保存最后一次文件摘要，同一份剪贴板下次贴出路径文字

use std::{
    io::{self, Read, Write},
    path::PathBuf,
    time::Duration,
};

pub(super) struct State {
    path: PathBuf,
    previous: Option<u32>,
    #[cfg(target_os = "windows")]
    _lock: crate::utils::fs_lock::Lock,
    #[cfg(target_os = "linux")]
    _lock: nix::fcntl::Flock<std::fs::File>,
}

impl State {
    /// 【贴图】【状态读取】无参数，锁定当前配置对应状态并有界读取摘要，返回事务；未初始化状态视为空
    pub fn open() -> io::Result<Self> {
        let config = crate::config::preferences::path()?;
        let identity = crc32fast::hash(config.to_string_lossy().as_bytes());
        let directory = dirs::state_dir()
            .or_else(dirs::data_local_dir)
            .ok_or_else(|| io::Error::other("user state directory is unavailable"))?
            .join("mark-shot/clipboard-pinning")
            .join(format!("{identity:08x}"));
        std::fs::create_dir_all(&directory)?;
        let path = directory.join("pin-clipboard-state.json");
        let file =
            crate::utils::platform::open_regular(&directory.join(".pin-clipboard.lock"), true)?;
        let lock = crate::utils::fs_lock::acquire(file, Duration::from_secs(1))?;
        let previous = crate::utils::platform::open_regular(&path, false)
            .ok()
            .and_then(|file| {
                let mut bytes = Vec::new();
                file.take(64).read_to_end(&mut bytes).ok()?;
                serde_json::from_slice::<Option<u32>>(&bytes).ok().flatten()
            });
        Ok(Self {
            path,
            previous,
            _lock: lock,
        })
    }
    /// 【贴图】【上次摘要】无参数，返回最后一次复制文件贴图摘要
    pub fn previous(&self) -> Option<u32> {
        self.previous
    }
    /// 【贴图】【成功提交】按 fingerprint 原子保存已交接内容的摘要，返回文件结果
    pub fn save(&mut self, fingerprint: Option<u32>) -> io::Result<()> {
        let mut file = tempfile::NamedTempFile::new_in(self.path.parent().unwrap())?;
        file.write_all(&serde_json::to_vec(&fingerprint)?)?;
        file.as_file().sync_all()?;
        file.persist(&self.path).map_err(|error| error.error)?;
        self.previous = fingerprint;
        Ok(())
    }
}
