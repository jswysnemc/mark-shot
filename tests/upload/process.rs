use super::*;
use std::{path::Path, sync::mpsc, time::Instant};

/// 【上传测试】【子进程记录】等待 path 的真实记录，返回字符串；没有实际完成的进程不能视为已启动
fn record(path: &Path) -> String {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Ok(text) = std::fs::read_to_string(path)
            && !text.is_empty()
        {
            return text;
        }
        assert!(
            Instant::now() < deadline,
            "upload command did not write receipt"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// 【上传测试】【命令文件】真实命令读取完整私有 PNG 和配置环境，完成后输入文件必须删除，结果使用原版纯 URL 回退
#[test]
fn command_receives_private_png_env_and_removes_file() {
    let directory = tempfile::tempdir().unwrap();
    let receipt = directory.path().join("receipt.json");
    let image_copy = directory.path().join("copy.png");
    let script = directory.path().join("receiver.py");
    std::fs::write(&script, "import json,os,pathlib,sys\np=pathlib.Path(sys.argv[3])\npathlib.Path(sys.argv[2]).write_bytes(p.read_bytes())\npathlib.Path(sys.argv[1]).write_text(json.dumps({'image':str(p),'mode':p.stat().st_mode&0o777,'token':os.getenv('TEST_TOKEN'),'args':sys.argv[3:]}))\nprint('https://images.example/command.png')\n").unwrap();
    let quote = mark_shot::features::external_actions::launch::shell_quote;
    let config = Upload {
        command: format!(
            "/usr/bin/python3 {} {} {} {{image}}",
            quote(&script.to_string_lossy()),
            quote(&receipt.to_string_lossy()),
            quote(&image_copy.to_string_lossy())
        ),
        env: std::collections::BTreeMap::from([("TEST_TOKEN".into(), "local-token".into())]),
        ..Default::default()
    };
    let image = include_bytes!("../fixtures/qt-export-image/case-17-source.png").to_vec();
    let result = run(config, image.clone()).unwrap();
    assert_eq!(result.url, "https://images.example/command.png");
    assert_eq!(std::fs::read(image_copy).unwrap(), image);
    let receipt: serde_json::Value = serde_json::from_str(&record(&receipt)).unwrap();
    assert_eq!(receipt["mode"], 0o600);
    assert_eq!(receipt["token"], "local-token");
    assert_eq!(receipt["args"].as_array().unwrap().len(), 1);
    assert!(
        !Path::new(receipt["image"].as_str().unwrap()).exists(),
        "upload input survived completion"
    );
}

/// 【上传测试】【失败命令】实际错误退出、无效 URL、超限管道和无占位符命令遵循失败或追加图片策略
#[test]
fn actual_commands_check_exit_output_limits_and_append_image() {
    for (command, expected) in [
        ("exit 4", upload::ErrorKind::Command),
        ("printf '%s' '{\"url\":\"\"}'", upload::ErrorKind::Protocol),
        (
            "/usr/bin/python3 -c 'print(\"x\"*1100000)'",
            upload::ErrorKind::Protocol,
        ),
        (
            "/usr/bin/python3 -c 'import sys; sys.stderr.write(\"x\"*600000)'",
            upload::ErrorKind::Protocol,
        ),
    ] {
        let error = run(
            Upload {
                command: command.into(),
                ..Default::default()
            },
            vec![1, 2, 3],
        )
        .unwrap_err();
        assert_eq!(error.kind, expected, "{command}");
    }
    let result = run(Upload { command: "/usr/bin/python3 -c 'import pathlib,sys; assert pathlib.Path(sys.argv[1]).read_bytes()==bytes([1,2,3]); print(\"https://images.example/appended\")'".into(), ..Default::default() }, vec![1, 2, 3]).unwrap();
    assert_eq!(result.url, "https://images.example/appended");
    let result = run(
        Upload {
            command: "/usr/bin/python3 -c 'import json; print(json.dumps({\"url\":\"file:///tmp/image.png\"}))'".into(),
            ..Default::default()
        },
        vec![1, 2, 3],
    )
    .unwrap();
    assert_eq!(result.url, "file:///tmp/image.png");
}

/// 【上传测试】【所属终止】按 cancel 取消或超时实际进程组，返回错误；父命令和后代必须退出，临时 PNG 必须删除
fn terminate_command(cancel: bool) {
    let directory = tempfile::tempdir().unwrap();
    let receipt = directory.path().join("receipt");
    let script = directory.path().join("wait.py");
    std::fs::write(&script, "import os,pathlib,sys,time\nchild=os.fork()\nif child==0:\n time.sleep(60)\nelse:\n pathlib.Path(sys.argv[1]).write_text(f'{os.getpid()} {child} {sys.argv[2]}')\n time.sleep(60)\n").unwrap();
    let quote = mark_shot::features::external_actions::launch::shell_quote;
    let config = Upload {
        command: format!(
            "/usr/bin/python3 {} {} {{imagePath}}",
            quote(&script.to_string_lossy()),
            quote(&receipt.to_string_lossy())
        ),
        timeout_ms: if cancel { 60000 } else { 200 },
        ..Default::default()
    };
    let (done, received) = mpsc::channel();
    let mut task = Task::start(config, vec![1, 2, 3], move || {
        let _ = done.send(());
    })
    .unwrap();
    let text = record(&receipt);
    let parts: Vec<_> = text.split_whitespace().collect();
    if cancel {
        task.cancel();
    }
    received.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(
        task.take_result().unwrap().unwrap_err().kind,
        if cancel {
            upload::ErrorKind::Canceled
        } else {
            upload::ErrorKind::Timeout
        }
    );
    task.shutdown();
    for pid in &parts[..2] {
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let status = std::fs::read_to_string(format!("/proc/{pid}/status"));
            if status.is_err()
                || status
                    .unwrap()
                    .lines()
                    .any(|line| line.starts_with("State:") && line.contains('Z'))
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "upload descendant remained running: {pid}"
            );
            // 1. 【上传测试】【退出观察】进程组信号已经发送，直接子进程退出不代表其他后代在同一个调度时刻退出
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    assert!(
        !Path::new(parts[2]).exists(),
        "cancel or timeout leaked upload PNG"
    );
}

/// 【上传测试】【主动回收】验证取消上传不等待一分钟命令退出，全部所属任务均终止
#[test]
fn cancellation_reaps_command_group() {
    terminate_command(true);
}

/// 【上传测试】【超时回收】验证总超时终止命令和后代，不留下私有图片或运行中任务
#[test]
fn timeout_reaps_command_group() {
    terminate_command(false);
}
