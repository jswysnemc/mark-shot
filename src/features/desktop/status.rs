//! 【桌面集成】【状态交付】按会话隔离服务锁和注册状态，设置窗口通过目录事件刷新

use notify::Watcher;
use serde::{Deserialize, Serialize};
use std::{
    hash::{Hash, Hasher},
    io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Status {
    pub shortcuts: Registration,
    pub tray: Registration,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(tag = "state", content = "detail", rename_all = "snake_case")]
pub enum Registration {
    #[default]
    Disabled,
    Registering,
    Paused,
    Registered(String),
    Unavailable,
    Failed(String),
    Stopped,
}

impl Registration {
    /// 【桌面集成】【状态文案】按chinese返回真实注册结果，平台诊断保留原始错误
    pub fn label(&self, chinese: bool) -> String {
        match self {
            Self::Disabled => if chinese { "未启用" } else { "Disabled" }.into(),
            Self::Paused => if chinese {
                "录入期间暂停监听"
            } else {
                "Paused while recording"
            }
            .into(),
            Self::Registering => if chinese {
                "正在注册，系统可能需要确认"
            } else {
                "Registering; system approval may be required"
            }
            .into(),
            Self::Registered(binding) if binding.is_empty() => {
                if chinese { "已注册" } else { "Registered" }.into()
            }
            Self::Registered(binding) => format!(
                "{}: {binding}",
                if chinese { "已注册" } else { "Registered" }
            ),
            Self::Unavailable => if chinese {
                "等待系统托盘区域；GNOME 需要托盘扩展"
            } else {
                "Waiting for a tray host; GNOME requires a tray extension"
            }
            .into(),
            Self::Failed(error) => format!(
                "{}: {error}",
                if chinese {
                    "注册失败"
                } else {
                    "Registration failed"
                }
            ),
            Self::Stopped => if chinese {
                "后台服务已停止"
            } else {
                "Background service stopped"
            }
            .into(),
        }
    }
}

/// 【桌面集成】【状态目录】按配置路径和显示会话计算私有目录，返回路径或创建错误
pub(super) fn directory(config: &Path) -> io::Result<PathBuf> {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    std::path::absolute(config)?.hash(&mut hash);
    for key in [
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "XDG_SESSION_TYPE",
        "DBUS_SESSION_BUS_ADDRESS",
    ] {
        std::env::var_os(key).hash(&mut hash);
    }
    let root = dirs::cache_dir()
        .ok_or_else(|| io::Error::other("cache directory unavailable"))?
        .join("mark-shot/desktop")
        .join(format!("{:016x}", hash.finish()));
    std::fs::create_dir_all(&root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(root)
}

/// 【桌面集成】【状态读取】按配置路径读取当前会话的状态，文件缺失返回空状态
pub fn read(config: &Path) -> Status {
    directory(config)
        .and_then(|root| {
            let bytes = std::fs::read(root.join("status.json"))?;
            let mut status: Status = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
            let lock = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(root.join("service.lock"))?;
            if lock.try_lock().is_ok() {
                for registration in [&mut status.shortcuts, &mut status.tray] {
                    if !matches!(registration, Registration::Disabled) {
                        *registration = Registration::Stopped;
                    }
                }
            }
            Ok(status)
        })
        .unwrap_or_default()
}

/// 【桌面集成】【原子状态】按root写入status，返回文件错误，不允许设置窗口看到半份JSON
pub(super) fn write(root: &Path, status: &Status) -> io::Result<()> {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new_in(root)?;
    serde_json::to_writer(&mut file, status).map_err(io::Error::other)?;
    file.flush()?;
    file.persist(root.join("status.json"))
        .map_err(io::Error::other)?;
    Ok(())
}

/// 【桌面集成】【状态监听】按config和wake订阅当前服务目录，返回负责释放系统监听的句柄
pub fn watch(
    config: &Path,
    wake: crate::backend::window::WindowWake,
) -> io::Result<notify::RecommendedWatcher> {
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.is_ok_and(|event| {
            event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove()
        }) {
            wake.notify();
        }
    })
    .map_err(io::Error::other)?;
    watcher
        .watch(&directory(config)?, notify::RecursiveMode::NonRecursive)
        .map_err(io::Error::other)?;
    Ok(watcher)
}
