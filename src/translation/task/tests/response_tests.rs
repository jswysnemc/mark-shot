//! 【翻译回归】【空结果】通过真实请求拒绝没有任何有效译文的成功响应

use super::*;

/// 【翻译回归】【拒绝假成功】无参数，覆盖空数组、陌生身份与空文本，不能把原文回填当作翻译成功
#[test]
fn no_usable_translation_is_reported_as_an_error() {
    for content in [
        r#"{"translations":[]}"#,
        r#"{"translations":[{"id":99,"text":"unrelated"}]}"#,
        r#"{"translations":[{"id":0,"text":"  "}]}"#,
    ] {
        let body = serde_json::json!({"choices":[{"message":{"content":content}}]}).to_string();
        let server = Server::start(Some((200, &body)));
        let mut task = Task::start_with(
            &config(&server),
            "original",
            "English",
            Path::new("/private/test-config"),
            WindowWake::detached().unwrap(),
            &|_| None,
        )
        .unwrap();
        let result = result(&mut task, Instant::now() + Duration::from_secs(2));
        assert!(
            result.is_err(),
            "empty provider output was reported as success: {result:?}"
        );
    }
}
