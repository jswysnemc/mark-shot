//! 【录制组件】【可执行文件】优先使用同包组件，开发环境允许系统工具

use std::path::PathBuf;

/// 【录制组件】【路径查找】按 name 返回随包或 PATH 中的可执行文件，缺失时说明需要完整录制包
pub fn executable(name: &str) -> Result<PathBuf, String> {
    let filename = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    if let Ok(exe) = std::env::current_exe()
        && let Some(root) = exe.parent()
    {
        let path = root.join("recording").join(&filename);
        if path.is_file() {
            return Ok(path);
        }
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&paths) {
            let path = directory.join(&filename);
            if path.is_file() {
                return Ok(path);
            }
        }
    }
    Err(format!(
        "缺少录制组件 {filename}，请使用完整构建包 / Recording component missing: {filename}"
    ))
}

/// 【录制组件】【子进程窗口】按 command 禁用 Windows 辅助控制台，其他平台保持默认
pub fn quiet(command: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    #[cfg(not(windows))]
    let _ = command;
}
