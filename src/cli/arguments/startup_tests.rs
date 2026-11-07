//! 【命令行回归】【原版启动参数】文件默认工具和截图范围别名在创建图形连接之前解析

use super::*;

/// 【命令行回归】【参数转换】按 values 解析原版启动参数，返回实际编辑计划，失败直接暴露解析错误
fn launch(values: &[&str]) -> Launch {
    let parsed = parse(values.iter().map(OsString::from), Instant::now()).unwrap();
    let Command::Editor(launch) = parsed.command else {
        panic!("expected editor launch");
    };
    launch
}

/// 【命令行回归】【原版别名】两种原版范围别名与现有拼写使用相同计划，无参数、无返回值
#[test]
fn original_capture_aliases_match_canonical_flags() {
    for option in ["--all-outputs", "--all-output"] {
        assert!(!launch(&[option]).one_monitor);
    }
    for option in ["--fullscreen", "--full-screen"] {
        assert!(launch(&[option]).fullscreen);
    }
}

/// 【命令行回归】【文件工具入口】原版独立文件工具与普通、全屏工具可共存，无参数、无返回值
#[test]
fn file_default_tool_is_accepted_with_other_startup_tools() {
    let launch = launch(&[
        "--default-tool",
        "pen",
        "--fullscreen-default-tool",
        "ruler",
        "--file-default-tool",
        "rectangle",
        "picture.png",
    ]);
    assert_eq!(launch.image, Some(PathBuf::from("picture.png")));
    assert_eq!(launch.default_tool, Some(crate::tools::Tool::Pen));
    assert_eq!(
        launch.fullscreen_default_tool,
        Some(crate::tools::Tool::Ruler)
    );
    assert_eq!(
        launch.file_default_tool,
        Some(crate::tools::Tool::Rectangle)
    );
}

/// 【命令行回归】【同名范围】同时提供新旧拼写不会遗留未知参数，别名保留与显示器参数的冲突规则，无参数、无返回值
#[test]
fn duplicate_alias_spellings_are_consumed_and_conflicts_remain_rejected() {
    let launch = launch(&[
        "--all-output",
        "--all-outputs",
        "--fullscreen",
        "--full-screen",
    ]);
    assert!(launch.fullscreen && !launch.one_monitor);
    assert!(
        parse(
            [OsString::from("--all-output"), OsString::from("--monitor")],
            Instant::now()
        )
        .is_err()
    );
}

/// 【命令行回归】【非法工具】无效文件工具在参数阶段返回工具错误，不把该参数识别为图片路径，无参数、无返回值
#[test]
fn invalid_file_tool_fails_before_graphics() {
    let error = parse(
        [
            OsString::from("--file-default-tool"),
            OsString::from("unknown"),
        ],
        Instant::now(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("unknown annotation tool"));
}
