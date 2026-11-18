//! 【门户连接回归】【运行时隔离】使用私有总线验证主题查询结束后门户连接不会卡死

#![cfg(target_os = "linux")]

use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    time::Duration,
};

struct PrivateBus(Child);

impl Drop for PrivateBus {
    /// 【门户连接回归】【总线回收】无参数，终止本测试创建的总线进程并等待退出，无返回值
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// 【门户连接回归】【临时主题】无参数，在隔离子进程中先执行真实主题查询，再验证新运行时的门户构造有界返回，无返回值
#[test]
#[ignore = "run by the private-bus parent test"]
fn transient_theme_runtime_does_not_poison_portal_connection() {
    let _ = mark_shot::theme::Mode::System.resolved();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    // 1. 【门户连接回归】【响应期限】私有总线没有门户服务，正常结果是及时报告缺少服务，旧连接会等待到超时
    let result = runtime.block_on(async {
        tokio::time::timeout(
            Duration::from_secs(2),
            ashpd::desktop::screencast::Screencast::new(),
        )
        .await
    });
    assert!(
        result.is_ok(),
        "theme query left the global portal connection on a stopped runtime"
    );
}

/// 【门户连接回归】【私有进程】无参数，建立独立总线并执行真实主题查询子测试，不修改用户环境或服务，无返回值
#[test]
fn theme_then_portal_uses_a_live_runtime_connection() {
    let child = Command::new("dbus-daemon")
        .args(["--session", "--nofork", "--print-address=1"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("dbus-daemon is required for Linux tests");
    let mut bus = PrivateBus(child);
    let mut address = String::new();
    BufReader::new(bus.0.stdout.take().unwrap())
        .read_line(&mut address)
        .unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "transient_theme_runtime_does_not_poison_portal_connection",
            "--ignored",
            "--nocapture",
        ])
        .env("DBUS_SESSION_BUS_ADDRESS", address.trim())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
