//! 【录制回归】【用户入口】验证录制命令不误入截图、导出或未实现路径
use mark_shot::cli::arguments::{Command, parse};
use std::{ffi::OsString, time::Instant};

/// 【录制回归】【命令解析】按 values 返回公共命令，非法参数由测试明确验证
fn command(values: &[&str]) -> mark_shot::AppResult<Command> {
    Ok(parse(values.iter().map(OsString::from), Instant::now())?.command)
}

/// 【录制回归】【控制入口】无参数，验证开始、选区、暂停、继续、停止与查询使用各自命令
#[test]
fn recording_commands_have_explicit_destinations() {
    assert!(matches!(
        command(&["--record"]).unwrap(),
        Command::Recording(_)
    ));
    assert!(
        matches!(command(&["--record-select"]).unwrap(), Command::Editor(launch) if launch.record_select)
    );
    for (flag, expected) in [
        ("--recording-status", "status"),
        ("--pause-recording", "pause"),
        ("--resume-recording", "resume"),
        ("--stop-recording", "stop"),
    ] {
        assert!(
            matches!(command(&[flag]).unwrap(), Command::RecordingControl(action) if action == expected)
        );
    }
    assert!(command(&["--record-target", "{}"]).is_err());
    assert!(command(&["--record", "--record-select"]).is_err());
}
