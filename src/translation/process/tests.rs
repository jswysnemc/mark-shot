//! 【翻译命令测试】【实际进程】验证 shell 参数引用、有限协议和私有输入，不运行用户配置中的命令

use super::*;
use serde_json::json;

/// 【翻译命令测试】【占位符引用】路径和目标中的 shell 元字符保持字面参数，缺少占位符时只追加输入文件，无参数、无返回值
#[test]
fn placeholders_are_quoted_and_preserve_original_append_rules() {
    assert_eq!(quote("a'b"), "'a'\\''b'");
    assert_eq!(
        command("tool", "/tmp/input file", "English", "/tmp/config"),
        "tool '/tmp/input file'"
    );
    assert_eq!(
        command(
            "tool {targetLanguage}",
            "/tmp/input",
            "$(not-a-command)",
            "/tmp/config"
        ),
        "tool '$(not-a-command)'"
    );
    assert_eq!(
        command(
            "tool {inputPath} {config}",
            "/tmp/input",
            "English",
            "/tmp/config file"
        ),
        "tool '/tmp/input' '/tmp/config file'"
    );
}

/// 【翻译命令测试】【占位符字面量】输入路径和语言包含其他占位符时保持字面参数，不递归展开用户数据，无参数、无返回值
#[test]
fn placeholder_values_are_not_reinterpreted_as_template_markers() {
    assert_eq!(
        command(
            "tool {input} {targetLanguage}",
            "/tmp/{targetLanguage}",
            "{config}",
            "/tmp/unrelated"
        ),
        "tool '/tmp/{targetLanguage}' '{config}'"
    );
}

/// 【翻译命令测试】【真实输入文件】实际子进程检查私有权限和原文快照，返回固定译文，结束后临时文件已经删除，无参数、无返回值
#[test]
fn command_receives_private_original_protocol_and_cleans_up_input() {
    let root = tempfile::tempdir().unwrap();
    let record = root.path().join("record.json");
    let script = "import json,os,sys; p=sys.argv[1]; data=json.load(open(p)); json.dump({'path':p,'mode':os.stat(p).st_mode & 511,'data':data,'target':sys.argv[2]},open(sys.argv[3],'w')); print(json.dumps({'tokens':[{'text':'translated'}]}))";
    let template = format!(
        "/usr/bin/python3 -c {} {{input}} {{targetLanguage}} {{config}}",
        quote(script)
    );
    let target = "English'; exit 7; #";
    let source = super::super::source_segments("alpha\n\n beta");
    let (_cancel, cancellation) = oneshot::channel();
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run(
            Provider::Command,
            &template,
            &source,
            target,
            &record,
            cancellation,
            Duration::from_secs(2),
        ))
        .unwrap();
    assert_eq!(result.text(), "translated");
    let recorded: Value = serde_json::from_slice(&std::fs::read(&record).unwrap()).unwrap();
    assert_eq!(recorded["mode"], 0o600);
    assert_eq!(recorded["target"], target);
    assert_eq!(recorded["data"]["targetLanguage"], target);
    assert_eq!(recorded["data"]["tokens"][1]["line"], 2);
    assert_eq!(
        recorded["data"]["tokens"][1]["box"],
        json!([0, 48.0, 1000.0, 20.0])
    );
    assert!(!Path::new(recorded["path"].as_str().unwrap()).exists());
}

/// 【翻译命令测试】【结果边界】格式错误和空译文不得作为成功，原版有效 tokens 可以保留段内换行，无参数、无返回值
#[test]
fn empty_or_invalid_command_output_is_not_success() {
    assert!(output(Provider::Command, b"not JSON").is_err());
    assert!(output(Provider::Helper, br#"{"tokens":[],"errors":["no result"]}"#).is_err());
    assert_eq!(
        output(
            Provider::Command,
            br#"{"tokens":[{"text":" first\nsecond "}]}"#
        )
        .unwrap()
        .text(),
        "first\nsecond"
    );
}
