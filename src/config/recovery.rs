//! 【应用设置】【故障修复】先保存原始备份，再修复可定位的错误行，禁止无备份覆盖
use std::{
    io,
    path::{Path, PathBuf},
};

/// 【应用设置】【备份修复】按 path 在共享锁内备份原文并移除非法 TOML 行，JSON 语法损坏恢复空对象，返回备份路径
pub fn repair(path: &Path) -> io::Result<PathBuf> {
    use io::Write;
    super::persistence::update(path, |text| {
        let parent = path.parent().unwrap_or(Path::new("."));
        let mut backup = tempfile::Builder::new()
            .prefix("config-broken-")
            .suffix(".bak")
            .tempfile_in(parent)?;
        backup.write_all(text.as_bytes())?;
        backup.as_file().sync_all()?;
        let (_, backup_path) = backup.keep().map_err(|error| error.error)?;
        let repaired = if super::document::is_json(path, text) {
            if super::legacy::decode(text).is_ok() {
                text.into()
            } else {
                "{}\n".into()
            }
        } else {
            repair_toml(text)?
        };
        Ok((repaired, backup_path))
    })
}

/// 【应用设置】【语法修复】按 text 逐行移除解析错误，保留合法字段、注释与未知键；无法定位返回错误
fn repair_toml(text: &str) -> io::Result<String> {
    let mut text = text.to_owned();
    for _ in 0..super::loading::MAX_BAD_LINES {
        let error = match toml::from_str::<super::Config>(&text) {
            Ok(_) => return Ok(text),
            Err(error) => error,
        };
        let span = error
            .span()
            .ok_or_else(|| io::Error::other(error.to_string()))?;
        let position = span.start.min(text.len());
        let start = text[..position].rfind('\n').map_or(0, |index| index + 1);
        let end = text[position..]
            .find('\n')
            .map_or(text.len(), |index| position + index);
        if start == end {
            return Err(io::Error::other("无法定位错误行；原文已备份"));
        }
        text.replace_range(start..end, "");
    }
    Err(io::Error::other("配置错误过多；原文已备份，请检查备份文件"))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【应用设置】【修复回归】无参数，语法错误修复保留原文备份、合法设置与未知字段
    #[test]
    fn repair_keeps_original_and_valid_settings() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        let text =
            "# kept\n[save]\ndirectory=\"unfinished\nfilename_format=\"%Y\"\n[custom]\nnumber=7\n";
        std::fs::write(&path, text).unwrap();
        let backup = repair(&path).unwrap();
        assert_eq!(std::fs::read_to_string(backup).unwrap(), text);
        let repaired = std::fs::read_to_string(&path).unwrap();
        assert!(repaired.contains("# kept") && repaired.contains("number=7"));
        assert_eq!(
            super::super::preferences::load(&path)
                .unwrap()
                .save
                .filename_format,
            "%Y"
        );
    }
}
