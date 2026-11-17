//! 【开机启动】【配置事务】更新当前用户的登录启动项，配置保存失败时恢复原注册

#[cfg(target_os = "linux")]
mod linux;
#[cfg(test)]
mod tests;
#[cfg(target_os = "windows")]
mod windows;

use std::{io, path::Path};

/// 【开机启动】【注册保存】按 path、enabled 更新系统注册，再执行 persist 保存配置，返回注册或保存错误
pub(crate) fn save(
    path: &Path,
    enabled: bool,
    persist: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    if cfg!(test) {
        return persist();
    }
    let previous = crate::config::preferences::load(path)?.desktop.autostart;
    transaction(enabled, previous, |value| update(path, value), persist)
}

/// 【开机启动】【启动同步】按 path 同步已保存的注册和程序位置，无返回值，失败记录错误但不阻止启动
pub(crate) fn reconcile(path: &Path) {
    if cfg!(test) {
        return;
    }
    let result = crate::config::preferences::load(path)
        .and_then(|config| update(path, config.desktop.autostart));
    if let Err(error) = result {
        log::error!("【开机启动】【启动同步】{error}");
    }
}

/// 【开机启动】【系统注册】按 path、enabled 使用持久化程序及配置路径更新用户启动项，返回系统错误
fn update(path: &Path, enabled: bool) -> io::Result<()> {
    let executable = crate::utils::self_executable::persistent_path()?;
    let config = std::path::absolute(path)?;
    #[cfg(target_os = "linux")]
    return linux::update(&executable, &config, enabled);
    #[cfg(target_os = "windows")]
    return windows::update(&executable, &config, enabled);
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "login startup is unsupported on this platform",
    ))
}

/// 【开机启动】【失败恢复】按 enabled、previous 调用 apply 和 persist，保存失败恢复旧开关，返回完整错误
fn transaction(
    enabled: bool,
    previous: bool,
    mut apply: impl FnMut(bool) -> io::Result<()>,
    persist: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    // 1. 【开机启动】【注册校验】注册失败时不写配置文件
    apply(enabled)?;
    // 2. 【开机启动】【配置提交】保存失败后撤销本次系统注册修改
    if let Err(error) = persist() {
        if let Err(rollback) = apply(previous) {
            return Err(io::Error::other(format!(
                "{error}; 启动项恢复失败 / Startup rollback failed: {rollback}"
            )));
        }
        return Err(error);
    }
    Ok(())
}
