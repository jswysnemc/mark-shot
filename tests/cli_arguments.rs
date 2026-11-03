use std::ffi::OsString;
use std::time::Instant;

use mark_shot::cli::arguments::{self, Command, Region};
use mark_shot::tools::Tool;
use mark_shot::types::Mode;

/// 【命令行测试】【参数构造】把测试字符串转换为系统参数并返回解析结果
fn parse(values: &[&str]) -> mark_shot::AppResult<arguments::Arguments> {
    arguments::parse(values.iter().map(OsString::from), Instant::now())
}

/// 【命令行测试】【默认入口】验证无参数时选择当前显示器并进入原生编辑器
#[test]
fn default_command_opens_current_display() {
    let Command::Editor(launch) = parse(&[]).unwrap().command else {
        panic!("expected editor")
    };
    assert_eq!(launch.mode, Mode::Editor);
    assert!(launch.one_monitor);
}

/// 【命令行测试】【历史入口】历史窗口与捕获互斥，重编辑只接受严格历史身份
#[test]
fn history_commands_are_explicit_and_exclusive() {
    assert!(matches!(
        parse(&["--history"]).unwrap().command,
        Command::History
    ));
    let name = "capture-20261103123456123-0123456789abcdef0123456789abcdef.png";
    assert!(matches!(
        parse(&["--history-edit", name]).unwrap().command,
        Command::HistoryEdit(_)
    ));
    for values in [
        vec!["--history", "--full"],
        vec!["--history", "--to", "c"],
        vec!["--history", "image.png"],
        vec!["--history", "--capture-to", "image.png"],
        vec!["--history", "--history-edit", name],
        vec!["--history-edit", "../image.png"],
    ] {
        assert!(parse(&values).is_err(), "{values:?}");
    }
}

/// 【命令行测试】【原版参数】验证显示器、逻辑区域、指针和文件名均保留到捕获任务
#[test]
fn parses_legacy_headless_flags() {
    let options = parse(&[
        "--capture-to",
        "/tmp/shots/",
        "--display",
        "DP-1",
        "--display",
        "DP-2",
        "--region",
        "-1920,0,1920,1080",
        "--include-cursor",
        "--output-name",
        "meeting",
    ])
    .unwrap();
    let Command::Capture(capture) = options.command else {
        panic!("expected capture")
    };
    assert_eq!(capture.displays, ["DP-1", "DP-2"]);
    assert_eq!(capture.region.unwrap().x, -1920);
    assert!(capture.include_cursor);
    assert_eq!(capture.output_name.as_deref(), Some("meeting"));
}

/// 【命令行测试】【非法输入】验证空区域、坐标溢出、非有限延迟和冲突命令均返回错误
#[test]
fn rejects_invalid_and_conflicting_options() {
    for region in ["0,0,0,2", "0,0,1", "0,0,-1,2", "2147483647,0,2,2"] {
        assert!(Region::parse(region).is_err());
    }
    for values in [
        vec!["--delay", "NaN"],
        vec!["--delay", "-1"],
        vec!["--capture-to", "shot.png", "--pin-clipboard"],
        vec!["--region", "0,0,1,1"],
        vec!["--full", "--ocr"],
        vec!["--recording-status"],
    ] {
        assert!(parse(&values).is_err(), "{values:?}");
    }
}

/// 【命令行测试】【编辑参数】验证本地图片、初始工具与颜色不丢失
#[test]
fn preserves_image_and_annotation_defaults() {
    let Command::Editor(launch) = parse(&[
        "--fullscreen",
        "--default-tool",
        "select",
        "--default-color",
        "#FF4D4D80",
        "/tmp/photo.png",
    ])
    .unwrap()
    .command
    else {
        panic!("expected editor")
    };
    assert_eq!(launch.default_tool, Some(Tool::Pick));
    assert_eq!(launch.image.unwrap().to_string_lossy(), "/tmp/photo.png");
    assert_eq!(launch.default_color.unwrap().alpha(), 128);
    assert!(launch.fullscreen);
}

/// 【命令行测试】【激光入口】验证原版全屏默认工具参数接受激光名称
#[test]
fn parses_the_fullscreen_laser_tool() {
    let Command::Editor(launch) = parse(&["--fullscreen", "--fullscreen-default-tool", "laser"])
        .unwrap()
        .command
    else {
        panic!("expected editor")
    };
    assert_eq!(launch.fullscreen_default_tool, Some(Tool::Laser));
}

/// 【命令行测试】【形状入口】标记与形状别名可作为普通或全屏初始工具
#[test]
fn parses_marker_tool_and_shape_alias() {
    for name in ["marker", "shape"] {
        let Command::Editor(launch) =
            parse(&["--fullscreen-default-tool", name, "--default-tool", name])
                .unwrap()
                .command
        else {
            panic!("expected editor")
        };
        assert_eq!(launch.default_tool, Some(Tool::Marker));
        assert_eq!(launch.fullscreen_default_tool, Some(Tool::Marker));
    }
}

/// 【命令行测试】【尺子入口】长度和角度共用尺子工具入口，接受 ruler 与 measure 名称
#[test]
fn parses_ruler_tool_and_measure_alias() {
    for name in ["ruler", "measure"] {
        let Command::Editor(launch) =
            parse(&["--fullscreen-default-tool", name, "--default-tool", name])
                .unwrap()
                .command
        else {
            panic!("expected editor")
        };
        assert_eq!(launch.default_tool, Some(Tool::Ruler));
        assert_eq!(launch.fullscreen_default_tool, Some(Tool::Ruler));
    }
}

/// 【命令行测试】【放大镜入口】原版四种放大镜名称均映射到持久双框工具，不替换光标取色放大镜
#[test]
fn parses_persistent_magnifier_tool_aliases() {
    for name in ["magnifier", "magnify", "loupe", "zoom"] {
        let Command::Editor(launch) =
            parse(&["--fullscreen-default-tool", name, "--default-tool", name])
                .unwrap()
                .command
        else {
            panic!("expected editor")
        };
        assert_eq!(launch.default_tool, Some(Tool::Magnifier));
        assert_eq!(launch.fullscreen_default_tool, Some(Tool::Magnifier));
    }
}

/// 【命令行测试】【独立序号】原版 number 和两个别名进入独立序号，不继续映射到旧编号箭头
#[test]
fn parses_standalone_number_tools_and_aliases() {
    for name in ["number", "serial", "sequence", "NUMBER"] {
        let Command::Editor(launch) =
            parse(&["--default-tool", name, "--fullscreen-default-tool", name])
                .unwrap()
                .command
        else {
            panic!("expected editor")
        };
        assert_eq!(launch.default_tool, Some(Tool::Number));
        assert_eq!(launch.fullscreen_default_tool, Some(Tool::Number));
    }
}

/// 【命令行测试】【旧箭头兼容】旧编号箭头名称继续对应独立保留的旧工具，不改变其几何或编号语义
#[test]
fn preserves_legacy_numbered_arrow_names() {
    for name in ["numbered", "numerated-arrow"] {
        let Command::Editor(launch) = parse(&["--default-tool", name]).unwrap().command else {
            panic!("expected editor")
        };
        assert_eq!(launch.default_tool, Some(Tool::NumeratedArrow));
    }
}

/// 【命令行测试】【辅助进程】验证内部剪贴板参数不触发截图，也不被当作图片路径
#[test]
fn clipboard_helper_bypasses_capture_arguments() {
    let Command::Internal { kind, args } = parse(&[
        "--clipboard-daemon",
        "text",
        "primary",
        "--config",
        "/tmp/config.toml",
    ])
    .unwrap()
    .command
    else {
        panic!("expected helper")
    };
    assert_eq!(kind, "--clipboard-daemon");
    assert_eq!(args, ["text", "primary"]);
}

/// 【命令行测试】【离线帮助】验证帮助与版本可在没有 Wayland 会话时解析
#[test]
fn help_and_version_are_pure_commands() {
    assert!(matches!(parse(&["--help"]).unwrap().command, Command::Help));
    assert!(matches!(
        parse(&["--version"]).unwrap().command,
        Command::Version
    ));
    assert!(matches!(parse(&["-v"]).unwrap().command, Command::Version));
}
