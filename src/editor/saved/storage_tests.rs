//! 【工具偏好测试】【平台持久化】验证没有 XDG 状态目录的平台仍保存独立粗细，并能替换已有偏好

use super::*;

/// 【尺子测试】【读数偏好】无参数，验证旧配置默认隐藏、单项设置往返及关闭最后一项后仍能恢复，无返回值
#[test]
fn ruler_readouts_restore_independently_and_legacy_settings_stay_quiet() {
    use crate::features::ruler::Readouts;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("tools.toml");
    std::fs::write(&path, "ruler_mode = 'angle'\n").unwrap();
    assert_eq!(load_at(&path).ruler_readouts, Readouts::default());
    for readouts in [
        Readouts {
            height: true,
            area: true,
            ..Default::default()
        },
        Readouts::default(),
    ] {
        let mut settings = ToolSettings::default();
        settings.ruler_readouts = readouts;
        storage::write(&path, &snapshot(&settings, &[])).unwrap();
        assert_eq!(load_at(&path).ruler_readouts, readouts);
    }
}

/// 【工具偏好测试】【Windows目录回退】无参数，模拟 Windows 无状态目录，实际保存和重新读取各工具粗细，无返回值
#[test]
fn windows_local_data_fallback_persists_and_restores_stroke_widths() {
    let directory = tempfile::tempdir().unwrap();
    let path = storage::select(None, Some(directory.path().to_owned()))
        .expect("Windows local app data must provide a persistent tools path");
    let file = File {
        stroke_widths: BTreeMap::from([("freehand".into(), 9.0), ("arrow".into(), 17.0)]),
        ..File::default()
    };
    storage::write(&path, &file).unwrap();
    let saved = load_at(&path);
    assert_eq!(saved.stroke(StrokeGroup::Freehand), Some(9.0));
    assert_eq!(saved.stroke(StrokeGroup::Arrow), Some(17.0));
    // 1. 【工具偏好测试】【再次保存】更新已有文件后恢复最新值，不沿用首次保存内容
    let file = File {
        stroke_widths: BTreeMap::from([("freehand".into(), 5.0), ("arrow".into(), 23.0)]),
        ..file
    };
    storage::write(&path, &file).unwrap();
    let saved = load_at(&path);
    assert_eq!(saved.stroke(StrokeGroup::Freehand), Some(5.0));
    assert_eq!(saved.stroke(StrokeGroup::Arrow), Some(23.0));
}

/// 【工具偏好测试】【现有状态目录】无参数，验证 Linux 状态目录仍优先使用且缺少所有目录时返回空值，无返回值
#[test]
fn existing_state_directory_remains_authoritative() {
    let state = PathBuf::from("state");
    let local = PathBuf::from("local");
    assert_eq!(
        storage::select(Some(state.clone()), Some(local)),
        Some(state.join("mark-shot/tools.toml"))
    );
    assert_eq!(storage::select(None, None), None);
}

/// 【工具偏好测试】【组合持久化】无参数，实际快照保存并恢复箭头独立属性、荧光笔模式与笔宽，无返回值
#[test]
fn production_snapshot_restores_arrow_attributes_and_highlighter_preferences() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("tools.toml");
    let mut settings = ToolSettings::default();
    settings.arrow_style = crate::features::arrow::ArrowStyle {
        tail: crate::features::arrow::ArrowTail::Pointed,
        ..crate::features::arrow::ArrowStyle::BidirectionalKde
    };
    settings.highlighter_style = crate::features::highlighter::HighlighterStyle::Freehand;
    settings.set_stroke(StrokeGroup::Highlighter, 19.0);
    storage::write(&path, &snapshot(&settings, &[])).unwrap();
    let saved = load_at(&path);
    assert_eq!(saved.arrow_style, Some(settings.arrow_style));
    assert_eq!(saved.highlighter_style, Some(settings.highlighter_style));
    assert_eq!(saved.stroke(StrokeGroup::Highlighter), Some(19.0));
}
