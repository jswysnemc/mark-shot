//! 【KDE窗口识别回归】【快照边界】验证活动桌面、窗口可见性、几何和层级

use super::*;
use serde_json::{Value, json};
use std::{io::BufRead, process::Command};

struct Bus(std::process::Child);

impl Drop for Bus {
    /// 【KDE窗口识别回归】【总线释放】无参数，仅结束测试建立的私有总线，返回值为空
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// 【KDE窗口识别回归】【私有总线】无参数，返回独立总线进程和连接地址，不连接用户会话
fn bus() -> (Bus, String) {
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
    (bus, address.trim().to_owned())
}

/// 【KDE窗口识别回归】【连接建立】按 address 连接测试总线，返回独立客户端连接
async fn connect(address: &str) -> Connection {
    zbus::connection::Builder::address(address)
        .unwrap()
        .build()
        .await
        .unwrap()
}

/// 【KDE窗口识别回归】【窗口数据】按 x 和 z 构造可见窗口记录，返回协议 JSON
fn record(x: i32, z: i32) -> Value {
    json!({"x":x,"y":20,"width":300,"height":200,"z":z,"eligible":true,
        "minimized":false,"hidden":false,"deleted":false,"on_desktop":true,"on_activity":true})
}

/// 【KDE窗口识别回归】【层级保留】无参数，验证窗口层级来自 KWin，重复几何只保留最高层级，无返回值
#[test]
fn visible_windows_preserve_stacking_and_deduplicate_geometry() {
    let windows =
        windows::decode(&json!([record(10, 7), record(50, 2), record(10, 12)]).to_string())
            .unwrap();
    assert_eq!(windows.len(), 2);
    assert_eq!(windows[0].rect.left(), 10.0);
    assert_eq!(windows[0].z_order, Some(12));
    assert_eq!(windows[1].z_order, Some(2));
}

/// 【KDE窗口识别回归】【不可见过滤】无参数，验证最小化、隐藏、关闭、其他桌面活动及面板不参与悬停，无返回值
#[test]
fn invisible_and_special_windows_are_excluded() {
    for (field, value) in [
        ("eligible", false),
        ("minimized", true),
        ("hidden", true),
        ("deleted", true),
        ("on_desktop", false),
        ("on_activity", false),
    ] {
        let mut window = record(10, 1);
        window[field] = json!(value);
        assert!(
            windows::decode(&json!([window]).to_string())
                .unwrap()
                .is_empty(),
            "{field}"
        );
    }
}

/// 【KDE窗口识别回归】【非法协议】无参数，验证超长、超量、异常几何及缺失字段不能形成错误候选，无返回值
#[test]
fn oversized_and_invalid_snapshots_cannot_create_candidates() {
    assert!(windows::decode(&" ".repeat(windows::MAX_REPLY + 1)).is_err());
    assert!(windows::decode(&serde_json::to_string(&vec![record(10, 1); 4097]).unwrap()).is_err());
    assert!(windows::decode("[{}]").is_err());
    let mut invalid = record(10, 1);
    invalid["width"] = json!(0);
    assert!(
        windows::decode(&json!([invalid]).to_string())
            .unwrap()
            .is_empty()
    );
}

/// 【KDE窗口识别回归】【回调来源】无参数，使用真实私有总线拒绝其他进程身份、超长及重复结果，返回值为空
#[tokio::test]
async fn callback_only_delivers_the_authenticated_first_reply() {
    let (_bus, address) = bus();
    let receiver_connection = connect(&address).await;
    let kwin = connect(&address).await;
    let stranger = connect(&address).await;
    let (sender, mut receiver) = oneshot::channel();
    receiver_connection
        .object_server()
        .at(
            protocol::PATH,
            protocol::Reply {
                owner: kwin.unique_name().unwrap().to_string(),
                sender: Mutex::new(Some(sender)),
            },
        )
        .await
        .unwrap();
    let destination = receiver_connection.unique_name().unwrap().as_str();
    assert!(
        stranger
            .call_method(
                Some(destination),
                protocol::PATH,
                Some("io.github.jswysnemc.MarkShot.WindowSnapshot"),
                "Submit",
                &("[]",)
            )
            .await
            .is_err()
    );
    assert!(
        kwin.call_method(
            Some(destination),
            protocol::PATH,
            Some("io.github.jswysnemc.MarkShot.WindowSnapshot"),
            "Submit",
            &(" ".repeat(windows::MAX_REPLY + 1),)
        )
        .await
        .is_err()
    );
    assert!(receiver.try_recv().is_err());
    kwin.call_method(
        Some(destination),
        protocol::PATH,
        Some("io.github.jswysnemc.MarkShot.WindowSnapshot"),
        "Submit",
        &("[]",),
    )
    .await
    .unwrap();
    assert_eq!(receiver.await.unwrap(), "[]");
    kwin.call_method(
        Some(destination),
        protocol::PATH,
        Some("io.github.jswysnemc.MarkShot.WindowSnapshot"),
        "Submit",
        &("[]",),
    )
    .await
    .unwrap();
}

struct Scripting {
    unloaded: std::sync::Arc<Mutex<Vec<String>>>,
}

#[zbus::interface(name = "org.kde.kwin.Scripting")]
impl Scripting {
    /// 【KDE窗口识别回归】【加载接口】按 file_path 和 plugin_name 验证原生大小写接口，返回测试脚本编号
    #[zbus(name = "loadScript")]
    fn load_script(&self, file_path: &str, plugin_name: &str) -> i32 {
        assert!(file_path.ends_with("snapshot.js"));
        assert_eq!(plugin_name, "mark-shot-window-fixture");
        17
    }

    /// 【KDE窗口识别回归】【记录卸载】按 plugin_name 保存实际卸载请求并返回成功，不触及真实 KWin
    #[zbus(name = "unloadScript")]
    fn unload_script(&self, plugin_name: &str) -> bool {
        self.unloaded.lock().unwrap().push(plugin_name.into());
        true
    }
}

/// 【KDE窗口识别回归】【守卫创建】按 connection 和 owner 创建独立文件守卫，返回守卫和临时路径
fn script(connection: Connection, owner: OwnedUniqueName) -> (Script, std::path::PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().to_owned();
    (
        Script {
            connection,
            owner,
            name: "mark-shot-window-fixture".into(),
            directory: Some(directory),
        },
        path,
    )
}

/// 【KDE窗口识别回归】【清理路径】无参数，通过真实私有总线验证完成及取消均卸载唯一脚本并删除临时目录，返回值为空
#[tokio::test]
async fn completed_and_cancelled_requests_release_their_script_and_files() {
    let (_bus, address) = bus();
    let kwin = connect(&address).await;
    let connection = connect(&address).await;
    let unloaded = std::sync::Arc::new(Mutex::new(Vec::new()));
    kwin.object_server()
        .at(
            "/Scripting",
            Scripting {
                unloaded: unloaded.clone(),
            },
        )
        .await
        .unwrap();
    let (mut completed, path) = script(connection.clone(), kwin.unique_name().unwrap().clone());
    let proxy = protocol::ScriptingProxy::builder(&connection)
        .destination(kwin.unique_name().unwrap().clone())
        .unwrap()
        .build()
        .await
        .unwrap();
    assert_eq!(
        proxy
            .load_script(path.join("snapshot.js").to_str().unwrap(), &completed.name)
            .await
            .unwrap(),
        17
    );
    completed.close().await.unwrap();
    assert!(!path.exists());
    assert_eq!(unloaded.lock().unwrap().len(), 1);
    drop(completed);
    let (cancelled, path) = script(connection, kwin.unique_name().unwrap().clone());
    drop(cancelled);
    tokio::time::timeout(Duration::from_millis(250), async {
        while path.exists() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        *unloaded.lock().unwrap(),
        ["mark-shot-window-fixture", "mark-shot-window-fixture"]
    );
}
