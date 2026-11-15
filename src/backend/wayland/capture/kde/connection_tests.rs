//! 【KWin连接测试】【后台交接】真实私有总线验证首帧预读后切换运行时仍能接收响应

use super::*;
use std::{io::BufRead, process::Command, time::Duration};

struct Bus(std::process::Child);

impl Drop for Bus {
    /// 【KWin连接测试】【总线清理】无参数，只结束本次启动的私有总线，不改变用户会话
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// 【KWin连接测试】【预读移交】无参数，预读运行时停止推进后在后台复用捕获器，必须完成真实总线请求并复用新连接
#[test]
fn primed_connection_reconnects_when_capture_moves_to_another_runtime() {
    let mut bus = Bus(Command::new("dbus-daemon")
        .args(["--session", "--nofork", "--print-address=1"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap());
    let mut address = String::new();
    std::io::BufReader::new(bus.0.stdout.take().unwrap())
        .read_line(&mut address)
        .unwrap();
    let address = address.trim().to_owned();
    let cached = RuntimeConnection::default();
    let frontend = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let first = frontend
        .block_on(cached.get_with(|| async {
            zbus::connection::Builder::address(address.as_str())?
                .build()
                .await
        }))
        .unwrap();
    // 1. 【KWin连接测试】【真实停顿】保留界面运行时但不再推进，模拟原生界面阻塞等待输入
    let backend = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    backend.block_on(async {
        let current = cached
            .get_with(|| async {
                zbus::connection::Builder::address(address.as_str())?
                    .build()
                    .await
            })
            .await
            .unwrap();
        let response = tokio::time::timeout(Duration::from_millis(500), async {
            zbus::fdo::DBusProxy::new(&current)
                .await?
                .list_names()
                .await
        })
        .await;
        assert!(
            response.is_ok(),
            "界面运行时停止推进后，后台总线响应仍被旧连接阻塞"
        );
        assert!(response.unwrap().is_ok());
        assert_ne!(first.unique_name(), current.unique_name());
        let reused = cached
            .get_with(|| async { panic!("同运行时不应重新连接") })
            .await
            .unwrap();
        assert_eq!(current.unique_name(), reused.unique_name());
    });
}
