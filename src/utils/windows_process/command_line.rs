//! 【Windows内部进程】【参数编码】保留UTF-16参数，按Windows运行库规则转义引号和反斜杠

use std::{ffi::OsStr, io, os::windows::ffi::OsStrExt, process::Command};

/// 【Windows内部进程】【字符串编码】按 value 返回零结尾UTF-16；内嵌零字符返回输入错误
pub(super) fn wide(value: &OsStr) -> io::Result<Vec<u16>> {
    let mut result: Vec<_> = value.encode_wide().collect();
    if result.contains(&0) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "embedded NUL"));
    }
    result.push(0);
    Ok(result)
}

/// 【Windows内部进程】【命令行编码】按 command 编码程序路径和全部参数，返回可供CreateProcessW修改的零结尾缓冲区
pub(super) fn encode(command: &Command) -> io::Result<Vec<u16>> {
    let program = wide(command.get_program())?;
    if program.contains(&u16::from(b'"')) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "quoted executable path",
        ));
    }
    let mut result = vec![u16::from(b'"')];
    result.extend_from_slice(&program[..program.len() - 1]);
    result.push(u16::from(b'"'));
    // 1. 【Windows内部进程】【独立参数】全部参数独立引用，空参数与尾部反斜杠不会合并到相邻参数
    for argument in command.get_args() {
        result.push(u16::from(b' '));
        quote(&mut result, argument)?;
    }
    result.push(0);
    if result.len() > 32767 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "command line is too long",
        ));
    }
    Ok(result)
}

/// 【Windows内部进程】【单参数引用】把 value 追加到 output，返回编码结果；引号前反斜杠加倍并转义引号
fn quote(output: &mut Vec<u16>, value: &OsStr) -> io::Result<()> {
    let value = wide(value)?;
    output.push(u16::from(b'"'));
    let mut slashes = 0;
    // 1. 【Windows内部进程】【反斜杠累计】保留连续反斜杠数量，在后续字符到达时统一编码
    for &unit in &value[..value.len() - 1] {
        if unit == u16::from(b'\\') {
            slashes += 1;
            continue;
        }
        output.extend(std::iter::repeat_n(u16::from(b'\\'), slashes));
        // 2. 【Windows内部进程】【引号转义】引号前反斜杠加倍，并增加一个反斜杠保留字面引号
        if unit == u16::from(b'"') {
            output.extend(std::iter::repeat_n(u16::from(b'\\'), slashes + 1));
        }
        output.push(unit);
        slashes = 0;
    }
    // 3. 【Windows内部进程】【尾部引用】尾部反斜杠加倍，避免吞掉参数结束引号
    output.extend(std::iter::repeat_n(u16::from(b'\\'), slashes * 2));
    output.push(u16::from(b'"'));
    Ok(())
}
