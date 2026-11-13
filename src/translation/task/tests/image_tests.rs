//! 【图片翻译测试】【真实任务】使用真实命令和 HTTP 验证原图段落快照、身份映射和完成通知

use super::*;
use crate::{
    features::image_translation::{BoxRect, SourceBlock},
    translation::request::Request,
};

/// 【图片翻译测试】【原图段落】返回两段真实坐标原文，不含虚拟行框或外部图片
fn blocks() -> Vec<SourceBlock> {
    [
        ("left original", BoxRect::new(23.5, 17.0, 140.0, 20.0)),
        ("right original", BoxRect::new(220.0, 19.0, 150.0, 20.0)),
    ]
    .into_iter()
    .map(|(text, rect)| SourceBlock {
        text: text.into(),
        rect,
        line_height: 20.0,
        line_pitch: 0.0,
        line_boxes: vec![rect],
    })
    .collect()
}

/// 【图片翻译测试】【命令坐标】真实命令接收原图坐标和私有输入，响应行号失配时按面积恢复，任务只通知一次并删除输入
#[test]
fn real_command_preserves_image_geometry_and_notifies_once_after_mapping() {
    let root = tempfile::tempdir().unwrap();
    let record = root.path().join("record.json");
    let script = "import json,os,sys; p=sys.argv[1]; data=json.load(open(p)); json.dump({'path':p,'mode':os.stat(p).st_mode & 511,'data':data},open(sys.argv[2],'w')); tokens=data['tokens']; tokens[0]['text']='left translated'; tokens[0]['line']=99; tokens[1]['text']='right translated'; tokens[1]['line']=-2; print(json.dumps({'tokens':list(reversed(tokens))}))";
    let config = Translation {
        provider: "plugin:missing".into(),
        command: format!(
            "exec /usr/bin/python3 -c '{}' {{input}} {{config}}",
            script.replace('\'', "'\\''")
        ),
        ..Default::default()
    };
    let (send, wake) = mpsc::channel();
    let mut task = Task::start_request_with(
        &config,
        Request::image(&blocks(), (400, 200)),
        "English",
        &record,
        move || {
            send.send(()).unwrap();
        },
        &|_| None,
    )
    .unwrap();
    let output = result(&mut task, Instant::now() + Duration::from_secs(2)).unwrap();
    wake.recv_timeout(Duration::from_secs(1)).unwrap();
    finished(&task);
    assert!(wake.try_recv().is_err());
    assert_eq!(
        output
            .segments
            .iter()
            .map(|segment| (segment.id, segment.text.as_str()))
            .collect::<Vec<_>>(),
        vec![(0, "left translated"), (1, "right translated")]
    );
    let recorded: serde_json::Value =
        serde_json::from_slice(&std::fs::read(record).unwrap()).unwrap();
    assert_eq!(recorded["mode"], 0o600);
    assert_eq!(
        recorded["data"]["tokens"][0]["box"],
        serde_json::json!([23.5, 17.0, 140.0, 20.0])
    );
    assert!(!Path::new(recorded["path"].as_str().unwrap()).exists());
    assert!(task.take_result().is_none());
}

/// 【图片翻译测试】【原生段落】真实原生请求按整段发送并保留身份，回复顺序变化不改变原图段落对应关系
#[test]
fn native_http_image_request_keeps_paragraph_ids_and_original_text() {
    let server = Server::start(Some((
        200,
        r#"{"choices":[{"message":{"content":"{\"translations\":[{\"id\":1,\"text\":\"right translated\"},{\"id\":0,\"text\":\"left translated\"}]}"}}]}"#,
    )));
    let mut task = Task::start_request_with(
        &config(&server),
        Request::image(&blocks(), (400, 200)),
        "English",
        Path::new("/private/config"),
        || {},
        &|_| None,
    )
    .unwrap();
    let request = server.seen.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(request.contains("left original"));
    assert!(request.contains("right original"));
    let output = result(&mut task, Instant::now() + Duration::from_secs(2)).unwrap();
    assert_eq!(output.segments[0].id, 0);
    assert_eq!(output.segments[0].text, "left translated");
    assert_eq!(output.segments[1].id, 1);
    finished(&task);
}

/// 【图片翻译测试】【响应资格】只有文字没有图像坐标的命令回复不能伪造图片翻译成功，原有纯文本协议仍可接收
#[test]
fn geometry_free_command_result_is_rejected_for_image_translation() {
    let config = Translation {
        command: "printf '%s' '{\"tokens\":[{\"text\":\"no geometry\"}]}'".into(),
        ..Default::default()
    };
    let mut task = Task::start_request_with(
        &config,
        Request::image(&blocks(), (400, 200)),
        "English",
        Path::new("/private/config"),
        || {},
        &|_| None,
    )
    .unwrap();
    assert_eq!(
        result(&mut task, Instant::now() + Duration::from_secs(2))
            .unwrap_err()
            .kind,
        ErrorKind::Protocol
    );
    finished(&task);
}
