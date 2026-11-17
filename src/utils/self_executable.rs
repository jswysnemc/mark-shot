//! 【辅助进程】【程序路径】识别 AppImage 临时挂载并切换为宿主包路径，防止父进程退出卸载代码页导致 SIGBUS

use std::{
    ffi::{OsStr, OsString},
    io,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Invocation {
    pub program: PathBuf,
    pub clear_appimage_env: bool,
    pub cleaned_library_path: Option<Option<OsString>>,
}

/// 【辅助进程】【命令构造】无参数，返回可跨越父进程生命周期独立运行的自身启动命令
pub(crate) fn command() -> io::Result<Command> {
    let invocation = resolve_current()?;
    Ok(build_command(invocation))
}

/// 【辅助进程】【持久路径】无参数，返回适合写入自启动项或长期引用的自身可执行文件路径
pub(crate) fn persistent_path() -> io::Result<PathBuf> {
    Ok(resolve_current()?.program)
}

/// 【辅助进程】【容器判定】无参数，返回当前进程是否运行在 AppImage 临时挂载目录内
pub(crate) fn uses_appimage() -> bool {
    resolve_current().is_ok_and(|invocation| invocation.clear_appimage_env)
}

/// 【辅助进程】【环境装配】按 invocation 构造清理过临时挂载变量的 Command，返回可执行命令对象
fn build_command(invocation: Invocation) -> Command {
    let mut command = Command::new(invocation.program);
    if invocation.clear_appimage_env {
        // 1. 【辅助进程】【挂载隔离】移除父级 AppImage 注入的挂载变量，让子实例独立建立并持有自有挂载生命周期
        for key in ["APPDIR", "ARGV0", "OWD"] {
            command.env_remove(key);
        }
        if let Some(library_path) = invocation.cleaned_library_path {
            match library_path {
                Some(value) => {
                    command.env("LD_LIBRARY_PATH", value);
                }
                None => {
                    command.env_remove("LD_LIBRARY_PATH");
                }
            }
        }
    }
    command
}

/// 【辅助进程】【当前解析】读取当前进程路径与环境变量并解析真实启动目标，返回 Invocation 或系统错误
fn resolve_current() -> io::Result<Invocation> {
    let current = std::env::current_exe()?;
    #[cfg(target_os = "linux")]
    {
        return Ok(resolve_from(
            &current,
            std::env::var_os("APPIMAGE").as_deref(),
            std::env::var_os("APPDIR").as_deref(),
            std::env::var_os("OWD").as_deref(),
            std::env::var_os("LD_LIBRARY_PATH").as_deref(),
        ));
    }
    #[cfg(not(target_os = "linux"))]
    Ok(Invocation {
        program: current,
        clear_appimage_env: false,
        cleaned_library_path: None,
    })
}

/// 【辅助进程】【路径裁决】按 current、appimage、appdir、owd 与 ld_library_path 计算启动目标，返回解析结果
#[cfg(any(target_os = "linux", test))]
fn resolve_from(
    current: &Path,
    appimage: Option<&OsStr>,
    appdir: Option<&OsStr>,
    owd: Option<&OsStr>,
    ld_library_path: Option<&OsStr>,
) -> Invocation {
    // 1. 【辅助进程】【归属核对】仅当自身二进制确实位于当前 APPDIR 内部时才切换为宿主 AppImage，避免误用外部启动器泄露的变量
    if let (Some(raw_image), Some(raw_dir)) = (appimage, appdir) {
        let mount = Path::new(raw_dir);
        if !raw_image.is_empty() && !raw_dir.is_empty() && inside_mount(current, mount) {
            let candidate = resolve_appimage_path(Path::new(raw_image), owd.map(Path::new));
            if candidate.is_file() {
                return Invocation {
                    program: candidate,
                    clear_appimage_env: true,
                    cleaned_library_path: Some(strip_mount_library_paths(ld_library_path, mount)),
                };
            }
        }
    }
    Invocation {
        program: current.to_path_buf(),
        clear_appimage_env: false,
        cleaned_library_path: None,
    }
}

/// 【辅助进程】【宿主定位】按 image 与 owd 将相对或符号链接的 AppImage 转为绝对文件路径，返回候选路径
#[cfg(any(target_os = "linux", test))]
fn resolve_appimage_path(image: &Path, owd: Option<&Path>) -> PathBuf {
    let joined = if image.is_absolute() {
        image.to_path_buf()
    } else if let Some(base) = owd.filter(|base| base.is_absolute()) {
        base.join(image)
    } else {
        std::path::absolute(image).unwrap_or_else(|_| image.to_path_buf())
    };
    std::fs::canonicalize(&joined).unwrap_or(joined)
}

/// 【辅助进程】【挂载包含】检查 executable 是否位于 mount 目录树内，兼容 /tmp 符号链接与规范化路径，返回布尔结果
#[cfg(any(target_os = "linux", test))]
fn inside_mount(executable: &Path, mount: &Path) -> bool {
    if executable.starts_with(mount) {
        return true;
    }
    let Ok(canonical_mount) = std::fs::canonicalize(mount) else {
        return false;
    };
    if executable.starts_with(&canonical_mount) {
        return true;
    }
    std::fs::canonicalize(executable)
        .is_ok_and(|canonical_exe| canonical_exe.starts_with(&canonical_mount))
}

/// 【辅助进程】【库路径清理】从 ld_library_path 剔除属于 mount 的临时目录，返回剩余路径串或空值
#[cfg(any(target_os = "linux", test))]
fn strip_mount_library_paths(ld_library_path: Option<&OsStr>, mount: &Path) -> Option<OsString> {
    let raw = ld_library_path?;
    let canonical_mount = std::fs::canonicalize(mount).ok();
    let kept: Vec<PathBuf> = std::env::split_paths(raw)
        .filter(|entry| {
            !entry.as_os_str().is_empty()
                && !entry.starts_with(mount)
                && !canonical_mount
                    .as_ref()
                    .is_some_and(|canonical| entry.starts_with(canonical))
        })
        .collect();
    if kept.is_empty() {
        None
    } else {
        std::env::join_paths(kept).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【辅助进程测试】【AppImage识别】位于 APPDIR 内的二进制切换至宿主 AppImage 并剥离临时库目录，外部启动器泄露变量不误触发
    #[test]
    fn appimage_mount_switches_to_host_bundle_and_strips_ephemeral_lib_dir() {
        let temp = tempfile::tempdir().unwrap();
        let bundle = temp.path().join("mark-shot.AppImage");
        std::fs::write(&bundle, b"#!/bin/sh\n").unwrap();
        let mount = temp.path().join(".mount_mark_1234");
        let binary = mount.join("usr/lib/mark-shot/mark-shot");
        std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
        std::fs::write(&binary, b"elf").unwrap();
        let custom_lib = temp.path().join("custom-lib");
        let ld = std::env::join_paths([mount.join("usr/lib"), custom_lib.clone()]).unwrap();

        let resolved = resolve_from(
            &binary,
            Some(OsStr::new("mark-shot.AppImage")),
            Some(mount.as_os_str()),
            Some(temp.path().as_os_str()),
            Some(&ld),
        );
        assert_eq!(
            resolved.program,
            std::fs::canonicalize(&bundle).unwrap_or(bundle)
        );
        assert!(resolved.clear_appimage_env);
        assert_eq!(
            resolved.cleaned_library_path,
            Some(Some(std::env::join_paths([custom_lib]).unwrap()))
        );

        // 1. 【辅助进程测试】【防误判】系统安装的二进制即使继承外部 AppImage 终端变量，仍必须调用自身路径
        let system_binary = temp.path().join("usr/bin/mark-shot");
        std::fs::create_dir_all(system_binary.parent().unwrap()).unwrap();
        std::fs::write(&system_binary, b"elf").unwrap();
        let untouched = resolve_from(
            &system_binary,
            Some(temp.path().join("mark-shot.AppImage").as_os_str()),
            Some(mount.as_os_str()),
            None,
            Some(&ld),
        );
        assert_eq!(untouched.program, system_binary);
        assert!(!untouched.clear_appimage_env);
    }

    /// 【辅助进程测试】【卸载存活】模拟父级挂载目录在交接后立即删除，独立子进程仍基于宿主 AppImage 正常运行
    #[cfg(target_os = "linux")]
    #[test]
    fn detached_child_survives_parent_mount_removal_with_clean_environment() {
        use std::{
            io::{BufRead, BufReader, Write},
            os::unix::fs::PermissionsExt,
            process::Stdio,
        };

        let temp = tempfile::tempdir().unwrap();
        let bundle = temp.path().join("mark-shot-test.AppImage");
        std::fs::write(
            &bundle,
            "#!/bin/sh\nprintf '\\n'\nread line\nprintf '%s|%s|%s\\n' \"${APPDIR:-}\" \"${LD_LIBRARY_PATH:-}\" \"$line\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&bundle, std::fs::Permissions::from_mode(0o755)).unwrap();

        let mount = temp.path().join(".mount_mark_ephemeral");
        let binary = mount.join("usr/lib/mark-shot/mark-shot");
        std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
        std::fs::write(&binary, b"ephemeral").unwrap();

        let invocation = resolve_from(
            &binary,
            Some(bundle.as_os_str()),
            Some(mount.as_os_str()),
            None,
            Some(mount.join("usr/lib").as_os_str()),
        );
        let mut command = build_command(invocation);
        // 1. 【辅助进程测试】【清理核对】确认临时挂载变量均已在子进程环境列表显式标记移除
        let removed: Vec<_> = command
            .get_envs()
            .filter_map(|(key, value)| value.is_none().then(|| key.to_string_lossy().into_owned()))
            .collect();
        for expected in ["APPDIR", "ARGV0", "OWD", "LD_LIBRARY_PATH"] {
            assert!(removed.iter().any(|item| item == expected), "{expected}");
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut pipes = crate::utils::helper_process::spawn(command).unwrap();
        let mut reader = BufReader::new(pipes.stdout.take().unwrap());
        let mut ready = String::new();
        reader.read_line(&mut ready).unwrap();
        assert_eq!(ready, "\n");

        // 2. 【辅助进程测试】【模拟卸载】父级临时挂载目录被整体移除后，子进程继续响应请求而不崩溃
        std::fs::remove_dir_all(&mount).unwrap();
        pipes
            .stdin
            .take()
            .unwrap()
            .write_all(b"settings-ok\n")
            .unwrap();
        let mut reply = String::new();
        reader.read_line(&mut reply).unwrap();
        assert_eq!(reply, "||settings-ok\n");
    }
}
