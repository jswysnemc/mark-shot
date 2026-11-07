//! 【主题通知测试】【原生事件】使用私有目录验证重复通知、双进程式监听与原子配置保存

use super::{Mode, Watch};

/// 【主题通知测试】【原子替换】无参数，两份独立来源均收到替换后的主题，文件多次替换后监听仍有效，无返回值
#[test]
fn independent_watchers_survive_atomic_config_replacement() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.toml");
    crate::config::interface_theme::save_at(&path, Mode::Light).unwrap();
    let first = Watch::new(&path, Mode::Light).unwrap();
    let second = Watch::new(&path, Mode::Light).unwrap();
    assert_eq!(first.changed().unwrap(), None);
    assert_eq!(second.changed().unwrap(), None);
    for mode in [Mode::Dark, Mode::Light, Mode::Dark] {
        crate::config::interface_theme::save_at(&path, mode).unwrap();
        assert_eq!(first.changed().unwrap(), Some(mode));
        assert_eq!(second.changed().unwrap(), Some(mode));
        assert_eq!(first.changed().unwrap(), None);
        assert_eq!(second.changed().unwrap(), None);
    }
}

/// 【主题通知测试】【首次创建】无参数，配置首次写入及直接修改均可通知，无关文件与其他业务更新不触发主题刷新，无返回值
#[test]
fn creation_and_direct_write_work_without_notifying_for_unrelated_edits() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("missing/config.toml");
    let watch = Watch::new(&path, Mode::Light).unwrap();
    assert_eq!(watch.changed().unwrap(), None);
    std::fs::write(&path, "[ui]\ntheme='dark'\n").unwrap();
    assert_eq!(watch.changed().unwrap(), Some(Mode::Dark));
    std::fs::write(path.with_file_name("other.toml"), "[ui]\ntheme='light'\n").unwrap();
    assert_eq!(watch.changed().unwrap(), None);
    std::fs::write(&path, "[ui]\ntheme='dark'\n[history]\nlimit=12\n").unwrap();
    assert_eq!(watch.changed().unwrap(), None);
    std::fs::remove_file(&path).unwrap();
    assert_eq!(watch.changed().unwrap(), Some(Mode::Light));
}

/// 【主题通知测试】【无效文件】无参数，无效中间内容不替换已保存主题，后续合法JSON替换继续通知，无返回值
#[test]
fn invalid_config_does_not_erase_the_last_seen_theme() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.json");
    std::fs::write(&path, r#"{"ui":{"theme":"dark"}}"#).unwrap();
    let watch = Watch::new(&path, Mode::Dark).unwrap();
    assert_eq!(watch.changed().unwrap(), None);
    std::fs::write(&path, "{").unwrap();
    assert!(watch.changed().is_err());
    assert_eq!(watch.changed().unwrap(), None);
    crate::config::interface_theme::save_at(&path, Mode::Light).unwrap_err();
    std::fs::write(&path, r#"{"ui":{"theme":"light"}}"#).unwrap();
    assert_eq!(watch.changed().unwrap(), Some(Mode::Light));
}
