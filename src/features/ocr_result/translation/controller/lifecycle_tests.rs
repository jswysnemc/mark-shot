//! 【OCR 翻译测试】【窗口退出】真实命令记录直接子进程、后代和输入路径，关闭窗口必须完成所属资源清理

use super::*;
use crate::backend::window::{WindowContent, WindowEvent};
use serde_json::Value;
use std::path::Path;

/// 【OCR 翻译测试】【阻塞任务】创建只访问私有配置的窗口及临时目录，返回两者；命令启动后代并记录资源后等待一分钟
fn running() -> (tempfile::TempDir, ResultWindow, Value) {
    let script = "import json,os,subprocess,sys,time; child=subprocess.Popen(['/bin/sleep','60']); json.dump({'pid':os.getpid(),'child':child.pid,'input':sys.argv[1]},open(sys.argv[2]+'.record.json','w')); time.sleep(60)";
    let command = format!(
        "exec /usr/bin/python -c '{}' {{input}} {{config}}",
        script.replace('\'', "'\\''")
    );
    let (root, mut window) = super::tests::window(&command);
    start(&mut window);
    let record = root.path().join("config.toml.record.json");
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if let Some(value) = std::fs::read(&record)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        {
            return (root, window, value);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("translation command did not record its owned resources")
}

/// 【OCR 翻译测试】【退出资源】按 record 验证直接子进程已回收、后代不再执行、临时输入已删除，无返回值，不等待应用退出后的补救
fn reclaimed(record: &Value) {
    let leader = record["pid"].as_u64().unwrap();
    let child = record["child"].as_u64().unwrap();
    assert!(!Path::new(&format!("/proc/{leader}")).exists());
    assert!(!Path::new(record["input"].as_str().unwrap()).exists());
    let deadline = Instant::now() + Duration::from_millis(300);
    loop {
        let live = std::fs::read_to_string(format!("/proc/{child}/stat"))
            .ok()
            .is_some_and(|stat| {
                stat.rsplit(") ")
                    .next()
                    .is_some_and(|tail| !tail.starts_with('Z'))
            });
        if !live {
            break;
        }
        assert!(Instant::now() < deadline, "owned descendant remains alive");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// 【OCR 翻译测试】【运行中关闭】窗口销毁前不取消任务，销毁返回时直接子进程与输入已回收，无参数、无返回值
#[test]
fn closing_a_running_translation_reclaims_resources_before_returning() {
    let (_root, window, record) = running();
    assert!(window.translation.state.busy());
    let started = Instant::now();
    drop(window);
    assert!(started.elapsed() < Duration::from_millis(700));
    reclaimed(&record);
}

/// 【OCR 翻译测试】【取消后立即关闭】交互取消保留所属句柄，随后关闭不等待唤醒也能回收资源，无参数、无返回值
#[test]
fn closing_immediately_after_cancel_keeps_the_cleanup_responsibility() {
    let (_root, mut window, record) = running();
    let started = Instant::now();
    cancel(&mut window, true);
    assert!(window.translation.task.is_none());
    assert_eq!(window.translation.retired.len(), 1);
    assert!(window.translation.retire_deadline.is_some());
    drop(window);
    assert!(started.elapsed() < Duration::from_millis(700));
    reclaimed(&record);
}

/// 【OCR 翻译测试】【取消后静止】保留的任务退出后通过实际唤醒删除句柄及临时计时，不留下永久五毫秒轮询，无参数、无返回值
#[test]
fn canceled_translation_retires_without_a_permanent_polling_deadline() {
    let (_root, mut window, record) = running();
    cancel(&mut window, true);
    let deadline = Instant::now() + Duration::from_secs(1);
    while !window.translation.retired.is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
        window.event(WindowEvent::Wake);
    }
    assert!(window.translation.retired.is_empty());
    assert!(window.translation.retire_deadline.is_none());
    assert!(window.translation.animation_deadline.is_none());
    reclaimed(&record);
}

/// 【OCR 翻译测试】【禁用字段焦点】语言输入启动翻译后焦点移到取消按钮，不保留禁用输入法目标，无参数、无返回值
#[test]
fn starting_from_language_moves_focus_to_the_enabled_action() {
    let (_root, mut window) = super::tests::window("sleep 0.3; exit 8");
    window.focus_control(FocusTarget::Language);
    start(&mut window);
    assert!(
        window.focus_target == FocusTarget::Button(super::super::super::layout::Control::Translate)
    );
    assert!(window.text_input_target().is_none());
}
