//! 【截图历史测试】【配置】覆盖独立策略和安全持久化

use super::*;
use std::fs;
#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;

/// 【截图历史测试】【原生解析】以 TOML 路径读取 text；返回约束后的历史策略或类型错误
fn parse(text: &str) -> io::Result<History> {
    parse_at(Path::new("config.toml"), text)
}

/// 【截图历史测试】【默认范围】空配置使用原版默认值，数量限制在一至二百
#[test]
fn settings_defaults_and_ranges() {
    assert_eq!(parse("").unwrap(), History::default());
    assert_eq!(
        parse("[history]\nlimit=0\nmax_size_mib=0\n").unwrap().limit,
        1
    );
    assert_eq!(parse("[history]\nlimit=999\n").unwrap().limit, 200);
    assert!(parse("[history]\nenabled='yes'\n").is_err());
}

/// 【截图历史测试】【字段保留】两项更新保留注释、其他配置和未知扩展，写入使用私有权限
#[test]
fn atomic_updates_preserve_comments_and_other_settings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "# personal settings\n[general]\nto='p'\n[custom]\nvalue=17\n[history]\nenabled=true # privacy\ndirectory='/custom'\n").unwrap();
    assert!(!update_at(&path, Update::Enabled(false)).unwrap().enabled);
    assert_eq!(update_at(&path, Update::Limit(9)).unwrap().limit, 9);
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("# personal settings") && text.contains("# privacy"));
    let document = text.parse::<toml::Table>().unwrap();
    assert_eq!(document["general"]["to"].as_str(), Some("p"));
    assert_eq!(document["custom"]["value"].as_integer(), Some(17));
    assert_eq!(document["history"]["directory"].as_str(), Some("/custom"));
    #[cfg(target_os = "linux")]
    assert_eq!(fs::metadata(path).unwrap().mode() & 0o777, 0o600);
}

/// 【截图历史测试】【失败保护】损坏配置和符号链接不能在更新时覆盖，链接目标保持原样
#[test]
fn invalid_config_and_links_are_not_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "[broken\n").unwrap();
    assert!(update_at(&path, Update::Enabled(false)).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), "[broken\n");
    fs::remove_file(&path).unwrap();
    let target = dir.path().join("target.toml");
    fs::write(&target, "# target\n").unwrap();
    crate::utils::test_files::link_file(&target, &path).unwrap();
    assert!(update_at(&path, Update::Limit(5)).is_err());
    assert_eq!(fs::read_to_string(target).unwrap(), "# target\n");
}
