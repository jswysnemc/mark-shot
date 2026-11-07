//! 【上传测试】【真实边界】本地 HTTP、命令进程与原版响应对照，不连接外部图床或读取用户凭据

#[path = "upload/config.rs"]
mod config;
#[path = "upload/http.rs"]
mod http;
#[path = "upload/process.rs"]
mod process;
#[path = "upload/reference.rs"]
mod reference;

use mark_shot::{
    config::upload::Upload,
    upload::{self, task::Task},
};
use std::{sync::mpsc, time::Duration};

/// 【上传测试】【任务等待】按 config 和 image 运行实际任务，返回一次性结果；完成必须真实通知，不通过状态轮询替代
fn run(config: Upload, image: Vec<u8>) -> upload::Result<upload::Output> {
    let (send, received) = mpsc::channel();
    let mut task = Task::start(config, image, move || {
        let _ = send.send(());
    })
    .unwrap();
    received
        .recv_timeout(Duration::from_secs(5))
        .expect("upload worker did not notify completion");
    let result = task
        .take_result()
        .expect("completion wake arrived without result");
    assert!(
        task.take_result().is_none(),
        "upload result delivered twice"
    );
    task.shutdown();
    assert!(
        received.try_recv().is_err(),
        "upload completion notified twice"
    );
    result
}
