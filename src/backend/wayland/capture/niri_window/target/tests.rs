//! 【niri 回归】【捕获目标】窗口稳定性、所属输出与关闭时资源回收

use super::*;
use crate::backend::wayland::niri::ipc::{self, Client};
use serde_json::Value;
use std::path::PathBuf;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

/// 【niri 回归】【同名窗口】稳定身份检查使用 ID 与工作区，拒绝切换至另一窗口，无参数、无返回值
#[test]
fn focus_and_workspace_changes_are_rejected() {
    let initial = Window {
        id: 7,
        workspace_id: Some(4),
        is_focused: true,
    };
    let mut current = Window {
        id: 7,
        workspace_id: Some(4),
        is_focused: true,
    };
    ensure_same_window(&initial, &current).unwrap();
    current.id = 8;
    assert!(ensure_same_window(&initial, &current).is_err());
    current.id = 7;
    current.workspace_id = Some(5);
    assert!(ensure_same_window(&initial, &current).is_err());
}

/// 【niri 回归】【所属输出】活动工作区定位实际输出比例，忽略列表顺序及其他显示器，无参数、无返回值
#[test]
fn window_workspace_selects_its_own_fractional_output() {
    let window = Window {
        id: 1,
        workspace_id: Some(9),
        is_focused: true,
    };
    let workspaces = vec![
        Workspace {
            id: 8,
            output: Some("first".into()),
        },
        Workspace {
            id: 9,
            output: Some("second".into()),
        },
    ];
    let mut outputs = HashMap::from([
        (
            "first".into(),
            Output {
                logical: Some(Logical { scale: 1.0 }),
            },
        ),
        (
            "second".into(),
            Output {
                logical: Some(Logical { scale: 1.5 }),
            },
        ),
    ]);
    assert_eq!(target_scale(&window, &workspaces, &outputs).unwrap(), 1.5);
    outputs.get_mut("second").unwrap().logical = None;
    assert!(target_scale(&window, &workspaces, &outputs).is_err());
}

/// 【niri 回归】【非法比例】有限正比例之外的输出值不得构造图片，无参数、无返回值
#[test]
fn invalid_or_missing_output_scale_is_rejected() {
    let window = Window {
        id: 1,
        workspace_id: Some(9),
        is_focused: true,
    };
    let workspaces = vec![Workspace {
        id: 9,
        output: Some("out".into()),
    }];
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY, 9.0] {
        let outputs = HashMap::from([(
            "out".into(),
            Output {
                logical: Some(Logical { scale }),
            },
        )]);
        assert!(target_scale(&window, &workspaces, &outputs).is_err());
    }
    assert!(target_scale(&window, &[], &HashMap::new()).is_err());
}

/// 【niri 回归】【窗口关闭】模拟目标在编码完成前关闭，固定 ID 不得回退且私有文件目录须回收，无参数、无返回值
#[tokio::test]
async fn closed_window_stops_capture_and_removes_temporary_directory() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("ipc.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let (probe, _) = UnixStream::pair().unwrap();
    let peer = ipc::tests::peer(&probe);
    let mut client = Client::connect(&path, peer).await.unwrap();
    let (main, _) = listener.accept().await.unwrap();
    let worker = tokio::spawn(async move {
        let (mut events, _) = listener.accept().await.unwrap();
        let mut main = BufReader::new(main);
        let mut event_reader = BufReader::new(&mut events);
        let mut line = String::new();
        event_reader.read_line(&mut line).await.unwrap();
        assert_eq!(line, "\"EventStream\"\n");
        event_reader
            .get_mut()
            .write_all(b"{\"Ok\":\"Handled\"}\n")
            .await
            .unwrap();
        line.clear();
        main.read_line(&mut line).await.unwrap();
        assert_eq!(line, "\"FocusedWindow\"\n");
        main.get_mut()
            .write_all(
                b"{\"Ok\":{\"FocusedWindow\":{\"id\":7,\"workspace_id\":4,\"is_focused\":true}}}\n",
            )
            .await
            .unwrap();
        line.clear();
        main.read_line(&mut line).await.unwrap();
        let request: Value = serde_json::from_str(&line).unwrap();
        let action = &request["Action"]["ScreenshotWindow"];
        assert_eq!(action["id"], 7);
        assert_eq!(action["show_pointer"], true);
        assert_eq!(action["write_to_disk"], true);
        let path = PathBuf::from(action["path"].as_str().unwrap());
        assert!(path.parent().unwrap().is_dir());
        main.get_mut()
            .write_all(b"{\"Ok\":\"Handled\"}\n")
            .await
            .unwrap();
        event_reader
            .get_mut()
            .write_all(b"{\"WindowClosed\":{\"id\":7}}\n")
            .await
            .unwrap();
        path
    });
    let target = Target {
        window: Window {
            id: 7,
            workspace_id: Some(4),
            is_focused: true,
        },
        scale: 1.5,
    };
    let result = super::super::legacy::capture(&mut client, &target, true).await;
    assert!(result.err().unwrap().to_string().contains("closed before"));
    let screenshot = worker.await.unwrap();
    assert!(!screenshot.parent().unwrap().exists());
}
