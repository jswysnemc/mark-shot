//! 【翻译命令测试】【退出边界】通过真实阻塞子进程验证超时、输出超限与临时文件回收

use super::*;
use serde_json::Value;

/// 【翻译命令测试】【完整记录】按 path 等待真实子进程完成记录写入，返回输入路径和 PID，不以文件存在代替内容已经写入
fn record(path: &Path) -> Value {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if let Some(value) = std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        {
            return value;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("command did not write its actual process record")
}

/// 【翻译命令测试】【统一资源断言】按 task 和 record 等待完成并核对预期错误、直接子进程及输入文件回收，无返回值
fn reclaimed(task: &mut Task, record: &Value, kind: ErrorKind) {
    assert_eq!(
        result(task, Instant::now() + Duration::from_secs(2))
            .unwrap_err()
            .kind,
        kind
    );
    finished(task);
    let pid = record["pid"].as_u64().unwrap();
    assert!(
        !Path::new(&format!("/proc/{pid}")).exists(),
        "owned direct child was not reaped"
    );
    assert!(!Path::new(record["input"].as_str().unwrap()).exists());
    assert!(task.take_result().is_none());
}

/// 【翻译命令测试】【记录命令】按 work 构造私有测试命令，子进程先记录自身和输入路径，再执行超时或超限行为
fn command(work: &str) -> String {
    let script = format!(
        "import json,os,sys,time; json.dump({{'pid':os.getpid(),'input':sys.argv[1]}},open(sys.argv[2],'w')); {work}"
    );
    format!(
        "exec /usr/bin/python3 -c '{}' {{input}} {{config}}",
        script.replace('\'', "'\\''")
    )
}

/// 【翻译命令测试】【真实超时回收】阻塞命令超时后直接子进程已消失且输入已删除，不依赖运行时销毁后的延时，无参数、无返回值
#[test]
fn timed_out_command_is_reaped_before_delivering_its_result() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("record.json");
    let config = Translation {
        command: command("time.sleep(60)"),
        timeout_ms: 1000,
        ..Default::default()
    };
    let mut task = Task::start_with(
        &config,
        "source",
        "English",
        &path,
        WindowWake::detached().unwrap(),
        &|_| None,
    )
    .unwrap();
    let recorded = record(&path);
    reclaimed(&mut task, &recorded, ErrorKind::Timeout);
}

/// 【翻译命令测试】【超量管道回收】stdout 或 stderr 超限时立即终止阻塞命令并回收，不等待长超时或继续缓存输出，无参数、无返回值
#[test]
fn excessive_stdout_and_stderr_reclaim_the_blocking_command() {
    for stream in ["stdout", "stderr"] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("record.json");
        let config = Translation {
            command: command(&format!(
                "sys.{stream}.write('x' * (8*1024*1024)); sys.{stream}.flush(); time.sleep(60)"
            )),
            timeout_ms: 10000,
            ..Default::default()
        };
        let mut task = Task::start_with(
            &config,
            "source",
            "English",
            &path,
            WindowWake::detached().unwrap(),
            &|_| None,
        )
        .unwrap();
        let recorded = record(&path);
        reclaimed(&mut task, &recorded, ErrorKind::Protocol);
    }
}

/// 【翻译命令测试】【立即取消无启动】取消消息已到达时不创建输入或启动命令，不产生记录文件，无参数、无返回值
#[test]
fn command_canceled_before_execution_has_no_process_side_effects() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("record.json");
    let (cancel, cancellation) = oneshot::channel();
    cancel.send(()).unwrap();
    let output = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(process::run(
            Provider::Command,
            &command("time.sleep(60)"),
            &source_segments("source"),
            "English",
            &path,
            cancellation,
            Duration::from_secs(2),
        ));
    assert_eq!(output.unwrap_err().kind, ErrorKind::Canceled);
    assert!(!path.exists());
}
