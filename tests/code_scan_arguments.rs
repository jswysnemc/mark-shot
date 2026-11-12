//! 【扫码测试】【命令入口】区分屏幕选区、图片窗口和机器可读输出，拒绝冲突动作

use mark_shot::cli::arguments::{Command, parse};
use std::{ffi::OsString, time::Instant};

/// 【扫码测试】【参数解析】按 values 返回正式命令或参数错误
fn command(values: &[&str]) -> mark_shot::AppResult<Command> {
    Ok(parse(values.iter().map(OsString::from), Instant::now())?.command)
}

/// 【扫码测试】【入口契约】无参数，验证图像不误入截图、命令行不误开窗口
#[test]
fn scanning_selects_the_requested_entry_point() {
    assert!(matches!(command(&["--scan"]).unwrap(), Command::Editor(launch) if launch.code_scan));
    assert!(
        matches!(command(&["--scan-code"]).unwrap(), Command::Editor(launch) if launch.code_scan)
    );
    assert!(
        matches!(command(&["--scan", "--scan-code"]).unwrap(), Command::Editor(launch) if launch.code_scan)
    );
    assert!(matches!(
        command(&["--scan", "code.png"]).unwrap(),
        Command::CodeScan { json: false, .. }
    ));
    assert!(matches!(
        command(&["--scan-json", "code.png"]).unwrap(),
        Command::CodeScan { json: true, .. }
    ));
    for values in [
        vec!["--scan-json"],
        vec!["--scan", "--scan-json", "code.png"],
        vec!["--scan", "--record-select"],
        vec!["--scan", "--ocr"],
        vec!["--scan", "--settings"],
        vec!["--scan", "--to", "c"],
        vec!["--scan-json", "--record", "code.png"],
    ] {
        assert!(command(&values).is_err(), "{values:?}");
    }
}
