//! 【辅助进程】【生命周期】长期辅助进程独立运行，父进程只回收短期启动进程

use std::{
    io,
    os::unix::process::CommandExt,
    process::{ChildStdin, ChildStdout, Command},
};

const CLOSE_RANGE_CLOEXEC: u32 = 4;

pub(crate) struct Pipes {
    pub stdin: Option<ChildStdin>,
    pub stdout: Option<ChildStdout>,
}

/// 【辅助进程】【独立启动】启动 command 并交接标准流；返回辅助进程的管道，不保留长期等待线程
pub(crate) fn spawn(mut command: Command) -> io::Result<Pipes> {
    // 1. 【辅助进程】【会话隔离】fork 后仅调用异步信号安全的系统接口，不访问锁、日志或堆分配
    unsafe {
        command.pre_exec(|| {
            if nix::libc::setsid() < 0 {
                return Err(io::Error::last_os_error());
            }
            let child = nix::libc::fork();
            if child < 0 {
                return Err(io::Error::last_os_error());
            }
            if child > 0 {
                nix::libc::_exit(0);
            }
            nix::libc::syscall(
                nix::libc::SYS_close_range,
                3u32,
                u32::MAX,
                CLOSE_RANGE_CLOEXEC,
            );
            Ok(())
        });
    }
    let mut launcher = command.spawn()?;
    let pipes = Pipes {
        stdin: launcher.stdin.take(),
        stdout: launcher.stdout.take(),
    };
    // 2. 【辅助进程】【启动回收】真正的辅助进程由系统回收，当前父进程只等待已经交接完成的启动进程
    if !launcher.wait()?.success() {
        return Err(io::Error::other("helper launcher failed"));
    }
    Ok(pipes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        process::Stdio,
    };

    /// 【辅助进程测试】【管道交接】短期启动进程退出后，实际辅助进程仍能完整读取输入并写出结果
    #[test]
    fn detached_helper_keeps_both_pipes() {
        let mut command = Command::new("/bin/cat");
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut pipes = spawn(command).unwrap();
        let data = b"helper stream\0binary\n".repeat(4096);
        let mut input = pipes.stdin.take().unwrap();
        let writer = std::thread::spawn(move || {
            input.write_all(&data).unwrap();
            data
        });
        let mut output = Vec::new();
        pipes
            .stdout
            .take()
            .unwrap()
            .read_to_end(&mut output)
            .unwrap();
        assert_eq!(output, writer.join().unwrap());
    }

    /// 【辅助进程测试】【会话交接】验证长驻进程已脱离父进程和原会话，标准错误仍写入指定文件
    #[test]
    fn helper_is_not_a_session_leader_or_a_waitable_child() {
        let errors = tempfile::tempfile().unwrap();
        let mut command = Command::new("/bin/sh");
        command
            .args([
                "-c",
                "printf '%s\\n' $$; printf 'helper stderr\\n' >&2; read value; printf '%s\\n' \"$value\"",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(errors.try_clone().unwrap());
        let mut pipes = spawn(command).unwrap();
        let mut output = BufReader::new(pipes.stdout.take().unwrap());
        let mut pid = String::new();
        output.read_line(&mut pid).unwrap();
        let pid: u32 = pid.trim().parse().unwrap();
        let status = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
        let fields: Vec<_> = status
            .rsplit_once(") ")
            .unwrap()
            .1
            .split_whitespace()
            .collect();
        assert_ne!(fields[1].parse::<u32>().unwrap(), std::process::id());
        assert_ne!(fields[2].parse::<u32>().unwrap(), pid);
        assert_eq!(fields[2], fields[3]);

        // 1. 【辅助进程测试】【结束授权】显式交接输入后才结束长驻进程，避免测试留下后台服务
        pipes.stdin.take().unwrap().write_all(b"finish\n").unwrap();
        let mut value = String::new();
        output.read_to_string(&mut value).unwrap();
        assert_eq!(value, "finish\n");
        assert_eq!(
            std::fs::read(format!(
                "/proc/self/fd/{}",
                std::os::fd::AsRawFd::as_raw_fd(&errors)
            ))
            .unwrap(),
            b"helper stderr\n"
        );
    }

    /// 【辅助进程测试】【执行错误】不存在的程序必须把 exec 错误返回父进程，不能报告假成功
    #[test]
    fn exec_errors_are_reported() {
        let error = spawn(Command::new("/does-not-exist/mark-shot-helper"))
            .err()
            .unwrap();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }
}
