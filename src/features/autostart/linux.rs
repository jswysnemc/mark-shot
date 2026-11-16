//! 【开机启动】【Linux注册】生成当前用户的 XDG 登录启动文件，保留带空格及特殊字符的路径

use std::{io, io::Write, path::Path};

/// 【开机启动】【Linux入口】按 executable、config、enabled 更新用户自动启动文件，返回目录或写入错误
pub(super) fn update(executable: &Path, config: &Path, enabled: bool) -> io::Result<()> {
    if enabled && std::env::var_os("FLATPAK_ID").is_some() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Flatpak 沙箱暂不支持开机启动 / Login startup is unavailable in Flatpak",
        ));
    }
    let directory = dirs::config_dir()
        .ok_or_else(|| io::Error::other("user configuration directory unavailable"))?;
    // 1. 【开机启动】【持久路径】AppImage 使用原始文件路径，避免登录后引用已经卸载的临时挂载目录
    let image = std::env::var_os("APPIMAGE");
    let executable = launch_path(executable, image.as_deref());
    write(
        &directory.join("autostart/io.github.jswysnemc.MarkShot.desktop"),
        executable,
        config,
        enabled,
    )
}

/// 【开机启动】【程序定位】按 executable 与 image 返回持久启动路径，未提供绝对 AppImage 路径时采用当前程序
fn launch_path<'a>(executable: &'a Path, image: Option<&'a std::ffi::OsStr>) -> &'a Path {
    image
        .map(Path::new)
        .filter(|path| path.is_absolute())
        .unwrap_or(executable)
}

/// 【开机启动】【路径编码】按 path 返回符合 Desktop Entry 字符串和 Exec 两层转义规则的引号参数
fn argument(path: &Path) -> io::Result<String> {
    let text = path
        .to_str()
        .ok_or_else(|| io::Error::other("startup path is not UTF-8"))?;
    if text.contains(['\n', '\r', '\0']) {
        return Err(io::Error::other(
            "startup path contains a control character",
        ));
    }
    let mut quoted = String::from("\"");
    for character in text.chars() {
        if matches!(character, '"' | '\\' | '$' | '`') {
            quoted.push('\\');
        }
        if character == '%' {
            quoted.push('%');
        }
        quoted.push(character);
    }
    quoted.push('"');
    Ok(quoted.replace('\\', "\\\\").replace('\t', "\\t"))
}

/// 【开机启动】【文件更新】按 path、executable、config、enabled 原子写入或删除启动文件，返回文件错误
fn write(path: &Path, executable: &Path, config: &Path, enabled: bool) -> io::Result<()> {
    if !enabled {
        return match std::fs::remove_file(path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        };
    }
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=Mark Shot\nExec={} --background --config {}\nIcon=io.github.jswysnemc.MarkShot\nTerminal=false\nStartupNotify=false\n",
        argument(executable)?,
        argument(config)?
    );
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("startup directory unavailable"))?;
    std::fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(entry.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【开机启动回归】【AppImage位置】无参数，验证登录启动引用原始镜像文件，普通程序仍使用实际执行文件
    #[test]
    fn appimage_uses_persistent_image_instead_of_mount_path() {
        let mounted = Path::new("/tmp/.mount_markshot/usr/lib/mark-shot/mark-shot");
        let image = std::ffi::OsStr::new("/home/user/Mark Shot.AppImage");
        assert_eq!(launch_path(mounted, Some(image)), Path::new(image));
        assert_eq!(launch_path(mounted, None), mounted);
        assert_eq!(
            launch_path(mounted, Some(std::ffi::OsStr::new("relative"))),
            mounted
        );
    }

    /// 【开机启动回归】【路径转义】无参数，验证空格、引号、百分号与命令字符不会拆分路径或形成字段码
    #[test]
    fn quotes_desktop_entry_arguments() {
        assert_eq!(
            argument(Path::new("/tmp/截图 100%/$app`\\\"")).unwrap(),
            "\"/tmp/截图 100%%/\\\\$app\\\\`\\\\\\\\\\\\\"\""
        );
        assert!(argument(Path::new("/tmp/app\nHidden=true")).is_err());
    }

    /// 【开机启动回归】【注册生命周期】无参数，验证启用、迁移程序路径、关闭和重复关闭均采用用户启动文件
    #[test]
    fn registers_refreshes_and_removes_user_entry() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("autostart/mark-shot.desktop");
        let config = Path::new("/tmp/my config.toml");
        write(&path, Path::new("/tmp/mark shot"), config, true).unwrap();
        let entry = std::fs::read_to_string(&path).unwrap();
        assert!(
            entry.contains("Exec=\"/tmp/mark shot\" --background --config \"/tmp/my config.toml\"")
        );
        write(&path, Path::new("/opt/mark-shot"), config, true).unwrap();
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("/opt/mark-shot")
        );
        write(&path, Path::new("/opt/mark-shot"), config, false).unwrap();
        write(&path, Path::new("/opt/mark-shot"), config, false).unwrap();
        assert!(!path.exists());
    }
}
