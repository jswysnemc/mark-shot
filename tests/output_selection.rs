use mark_shot::backend::output_selection::indices;

/// 【显示器测试】【选择规则】验证活动显示器、指定名称和全桌面捕获的索引一致
#[test]
fn display_selection_is_deterministic() {
    let names = vec!["DP-1".into(), "HDMI-A-1".into()];
    assert_eq!(indices(&names, &[], false, Some(1)).unwrap(), [1]);
    assert_eq!(indices(&names, &[], false, None).unwrap(), [0]);
    assert_eq!(indices(&names, &[], true, None).unwrap(), [0, 1]);
    assert_eq!(
        indices(&names, &["HDMI-A-1".into(), "DP-1".into()], false, None).unwrap(),
        [1, 0]
    );
    assert!(indices(&names, &["missing".into()], true, None).is_err());
    assert!(indices(&[], &[], true, None).is_err());
}
