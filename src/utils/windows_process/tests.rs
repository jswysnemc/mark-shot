//! 【Windows内部进程测试】【真实交接】子进程读取实际启动标志，验证管道、环境覆盖、文件输出与取消回收

use super::*;
use std::{
    io::{Read, Write},
    os::windows::ffi::OsStringExt,
};
use windows_sys::Win32::System::Threading::{GetStartupInfoW, STARTUPINFOW};

const PROBE: &str = "utils::windows_process::tests::child_probe";

/// 【Windows内部进程测试】【子进程样本】无参数，仅由带模式环境变量的测试子进程读取启动标志并交接数据，无返回值
#[test]
fn child_probe() {
    // 1. 【Windows内部进程测试】【启动信息】只在本次样本模式下核验真实启动标志
    let Ok(mode) = std::env::var("MARK_SHOT_PROCESS_PROBE") else {
        return;
    };
    let mut startup: STARTUPINFOW = unsafe { std::mem::zeroed() };
    unsafe { GetStartupInfoW(&mut startup) };
    assert_ne!(startup.dwFlags & STARTF_FORCEOFFFEEDBACK, 0);
    assert_ne!(startup.dwFlags & STARTF_USESTDHANDLES, 0);
    // 2. 【Windows内部进程测试】【取消样本】交付就绪信号后等待父进程终止
    if mode == "sleep" {
        println!("process-probe-ready");
        std::io::stdout().flush().unwrap();
        loop {
            std::thread::park();
        }
    }
    // 3. 【Windows内部进程测试】【数据样本】核验环境和工作目录，回传输入并写入错误流
    assert_eq!(std::env::var("PATH").unwrap(), "process-probe-path");
    assert!(std::env::var_os("COMSPEC").is_none());
    assert_eq!(
        std::env::current_dir().unwrap(),
        std::path::PathBuf::from(std::env::var_os("MARK_SHOT_PROCESS_DIRECTORY").unwrap())
    );
    let mut data = Vec::new();
    std::io::stdin().read_to_end(&mut data).unwrap();
    std::io::stdout()
        .write_all(b"process-probe-begin\n")
        .unwrap();
    std::io::stdout().write_all(&data).unwrap();
    std::io::stdout()
        .write_all(b"\nprocess-probe-end\n")
        .unwrap();
    eprintln!("process-probe-error");
}

/// 【Windows内部进程测试】【命令样本】按 mode 与 directory 构造当前测试程序命令，返回带环境覆盖的独立调用
fn probe(mode: &str, directory: &std::path::Path) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", PROBE, "--nocapture", "--test-threads=1"])
        .env("MARK_SHOT_PROCESS_PROBE", mode)
        .env("MARK_SHOT_PROCESS_DIRECTORY", directory)
        .env("paTH", "process-probe-path")
        .env_remove("COMSPEC")
        .current_dir(directory);
    command
}

/// 【Windows内部进程测试】【真实管道】无参数，验证子进程实际启动标志、环境和超过管道容量的数据交接，无返回值
#[test]
fn native_spawn_disables_feedback_and_preserves_pipes_environment_and_directory() {
    // 1. 【Windows内部进程测试】【准备样本】使用含中文与空格的目录及独立错误输出文件
    let directory = tempfile::Builder::new()
        .prefix("进程 测试 ")
        .tempdir()
        .unwrap();
    let errors = tempfile::tempfile().unwrap();
    // 2. 【Windows内部进程测试】【管道交接】并行写入和读取大于管道容量的数据，避免读写互相等待
    let mut child = spawn(
        &probe("echo", directory.path()),
        0,
        Streams {
            stdin: Stream::Piped,
            stdout: Stream::Piped,
            stderr: Some(errors.try_clone().unwrap()),
        },
    )
    .unwrap();
    let data = b"pipeline-payload\0\n".repeat(8192);
    let mut input = child.stdin.take().unwrap();
    let expected = data.clone();
    let writer = std::thread::spawn(move || {
        input.write_all(&data).unwrap();
    });
    let mut output = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut output)
        .unwrap();
    writer.join().unwrap();
    // 3. 【Windows内部进程测试】【结果核验】检查退出状态、完整负载和独立错误输出
    assert!(
        child.wait().unwrap().success(),
        "{}",
        String::from_utf8_lossy(&output)
    );
    let begin = b"process-probe-begin\n";
    let offset = output
        .windows(begin.len())
        .position(|slice| slice == begin)
        .unwrap()
        + begin.len();
    assert_eq!(&output[offset..offset + expected.len()], expected);
    assert_eq!(child.try_wait().unwrap(), Some(child.wait().unwrap()));
    assert!(child.kill().is_ok());
    let mut errors = errors;
    use std::io::{Seek, SeekFrom};
    errors.seek(SeekFrom::Start(0)).unwrap();
    let mut error_text = String::new();
    errors.read_to_string(&mut error_text).unwrap();
    assert!(error_text.contains("process-probe-error"));
}

/// 【Windows内部进程测试】【文件交接】无参数，验证识别任务使用的文件标准流和等待时关闭输入管道的EOF行为，无返回值
#[test]
fn native_spawn_preserves_file_streams_and_wait_closes_stdin() {
    use std::io::{Seek, SeekFrom};
    // 1. 【Windows内部进程测试】【文件准备】创建独立输入输出文件并写入样本
    let directory = tempfile::tempdir().unwrap();
    let mut input = tempfile::tempfile().unwrap();
    input.write_all(b"file-payload").unwrap();
    input.seek(SeekFrom::Start(0)).unwrap();
    let mut output = tempfile::tempfile().unwrap();
    // 2. 【Windows内部进程测试】【文件交接】等待子进程结束后读取完整输出
    let mut child = spawn(
        &probe("echo", directory.path()),
        0,
        Streams {
            stdin: Stream::File(input),
            stdout: Stream::File(output.try_clone().unwrap()),
            stderr: None,
        },
    )
    .unwrap();
    assert!(child.wait().unwrap().success());
    output.seek(SeekFrom::Start(0)).unwrap();
    let mut text = String::new();
    output.read_to_string(&mut text).unwrap();
    assert!(text.contains("file-payload"));
    // 3. 【Windows内部进程测试】【输入结束】等待入口关闭父端输入管道，使子进程收到结束标志
    let mut child = spawn(
        &probe("echo", directory.path()),
        0,
        Streams {
            stdin: Stream::Piped,
            stdout: Stream::Null,
            stderr: None,
        },
    )
    .unwrap();
    assert!(child.wait().unwrap().success());
}

/// 【Windows内部进程测试】【取消回收】无参数，验证运行中的子进程可终止，非阻塞领取不会误判为已结束，无返回值
#[test]
fn native_spawn_can_cancel_a_running_process() {
    use std::io::{BufRead, BufReader};
    // 1. 【Windows内部进程测试】【取消准备】创建保持运行并交付就绪信号的独立子进程
    let directory = tempfile::tempdir().unwrap();
    let mut child = spawn(
        &probe("sleep", directory.path()),
        0,
        Streams {
            stdin: Stream::Null,
            stdout: Stream::Piped,
            stderr: None,
        },
    )
    .unwrap();
    // 2. 【Windows内部进程测试】【等待就绪】读取真实就绪信号，保证终止发生在子进程开始运行后
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    loop {
        assert_ne!(output.read_line(&mut line).unwrap(), 0);
        if line.contains("process-probe-ready") {
            break;
        }
        line.clear();
    }
    // 3. 【Windows内部进程测试】【终止核验】运行状态不能误判为结束，终止后必须取得失败退出码
    assert!(child.try_wait().unwrap().is_none());
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
}

/// 【Windows内部进程测试】【系统参数解析】无参数，通过Windows解析器验证空值、空格、引号、尾部反斜杠和UTF-16代码单元，无返回值
#[test]
fn command_line_round_trips_windows_arguments() {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{Foundation::LocalFree, UI::Shell::CommandLineToArgvW};
    // 1. 【Windows内部进程测试】【参数准备】覆盖空值、引号、尾部反斜杠和未配对UTF-16代码单元
    let arguments = [
        std::ffi::OsString::from(""),
        "a b".into(),
        "quote\"value".into(),
        "back\\\"quote\\".into(),
        "目录\\".into(),
        std::ffi::OsString::from_wide(&[0xd800, 0x61]),
    ];
    let mut command = Command::new("C:\\目录 空格\\mark-shot.exe");
    command.args(&arguments);
    // 2. 【Windows内部进程测试】【系统解析】将编码结果交给系统参数解析器
    let encoded = command_line::encode(&command).unwrap();
    let mut count = 0;
    let parsed = unsafe { CommandLineToArgvW(encoded.as_ptr(), &mut count) };
    assert!(!parsed.is_null());
    assert_eq!(count as usize, arguments.len() + 1);
    // 3. 【Windows内部进程测试】【逐项核验】比较每个参数的完整代码单元，并释放系统解析结果
    for (index, expected) in arguments.iter().enumerate() {
        let raw = unsafe { *parsed.add(index + 1) };
        let mut length = 0;
        unsafe {
            while *raw.add(length) != 0 {
                length += 1;
            }
        }
        assert_eq!(
            unsafe { std::slice::from_raw_parts(raw, length) },
            expected.encode_wide().collect::<Vec<_>>()
        );
    }
    unsafe { LocalFree(parsed.cast()) };
}

/// 【Windows内部进程测试】【输入拒绝】无参数，验证零字符、超长命令与不存在程序返回错误，无返回值
#[test]
fn invalid_commands_are_rejected_without_starting_a_child() {
    // 1. 【Windows内部进程测试】【零字符拒绝】内嵌零字符必须在调用系统前返回输入错误
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.arg("embedded\0value");
    assert_eq!(
        command_line::encode(&command).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    // 2. 【Windows内部进程测试】【长度拒绝】超过系统命令行容量的输入不能启动进程
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.arg("a".repeat(32767));
    assert!(command_line::encode(&command).is_err());
    // 3. 【Windows内部进程测试】【系统错误】不存在的程序返回文件缺失错误
    let command = Command::new("C:\\does-not-exist\\mark-shot.exe");
    let error = spawn(
        &command,
        0,
        Streams {
            stdin: Stream::Null,
            stdout: Stream::Null,
            stderr: None,
        },
    )
    .err()
    .unwrap();
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}
