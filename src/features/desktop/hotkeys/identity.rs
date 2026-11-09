//! 【全局快捷键】【门户身份】为未安装桌面文件的独立二进制补充用户级身份，不设置登录自启

use std::{io::Write, path::PathBuf};

pub(super) const APP_ID: &str = "io.github.jswysnemc.MarkShot";

/// 【全局快捷键】【身份登记】按connection登记主机身份，短暂等待桌面目录监视器发现新文件，无返回值
pub(super) async fn register(connection: &zbus::Connection) -> Result<(), ashpd::Error> {
    for attempt in 0..20 {
        match ashpd::register_host_app_with_connection(connection.clone(), APP_ID.parse()?).await {
            Err(error) if attempt < 19 && error.to_string().contains("App info not found") => {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            result => return result,
        }
    }
    unreachable!()
}

/// 【全局快捷键】【身份文件】无参数，保留已安装身份；缺失时原子建立隐藏桌面条目，返回文件错误
pub(super) fn ensure() -> crate::AppResult {
    let data = dirs::data_dir().ok_or("application data directory unavailable")?;
    let filename = format!("{APP_ID}.desktop");
    let mut locations = vec![data.clone()];
    locations.extend(std::env::split_paths(
        &std::env::var_os("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into()),
    ));
    if locations
        .iter()
        .any(|directory| directory.join("applications").join(&filename).is_file())
    {
        return Ok(());
    }
    let applications = data.join("applications");
    std::fs::create_dir_all(&applications)?;
    let executable = std::env::current_exe()?;
    let executable = executable.to_str().ok_or("executable path is not UTF-8")?;
    if executable.contains(['\n', '\r']) {
        return Err("executable path contains a line break".into());
    }
    let mut quoted = String::from("\"");
    for character in executable.chars() {
        if matches!(character, '"' | '\\' | '$' | '`') {
            quoted.push('\\');
        }
        if character == '%' {
            quoted.push('%');
        }
        quoted.push(character);
    }
    quoted.push('"');
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=Mark Shot\nExec={} --settings\nIcon=mark-shot\nNoDisplay=true\nTerminal=false\n",
        quoted.replace('\\', "\\\\")
    );
    create_once(applications.join(filename), entry.as_bytes())?;
    let icons = data.join("icons/hicolor/scalable/apps");
    std::fs::create_dir_all(&icons)?;
    create_once(
        icons.join("mark-shot.svg"),
        include_bytes!("../../../../assets/hicolor/scalable/apps/mark-shot.svg"),
    )?;
    Ok(())
}

/// 【全局快捷键】【身份保存】按path与bytes原子创建缺失文件，保留已有用户文件，返回写入结果
fn create_once(path: PathBuf, bytes: &[u8]) -> std::io::Result<()> {
    let mut temporary = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    temporary.write_all(bytes)?;
    match temporary.persist_noclobber(path) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.error),
    }
}
