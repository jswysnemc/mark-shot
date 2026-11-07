use super::*;

/// 【上传测试】【生产样本】逐项比较只读 Qt 配置和 Python helper 实际输出，不以手写期望替代参考实现
#[test]
fn frozen_qt_config_and_helper_bytes_match() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/qt-upload.json")).unwrap();
    assert_eq!(fixture["configs"].as_array().unwrap().len(), 12);
    for case in fixture["configs"].as_array().unwrap() {
        let actual = Upload::from_root(&case["input"]);
        assert_eq!(actual.command, case["command"].as_str().unwrap());
        assert_eq!(actual.timeout_ms, case["timeout_ms"].as_u64().unwrap());
        assert_eq!(serde_json::to_value(&actual.env).unwrap(), case["env"]);
    }
    for case in fixture["extract"].as_array().unwrap() {
        assert_eq!(
            upload::request::extract(&case["input"], case["path"].as_str().unwrap()),
            case["output"].as_str().unwrap()
        );
    }
    assert_eq!(fixture["results"].as_array().unwrap().len(), 8);
    for case in fixture["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| matches!(case["kind"].as_str(), Some("json-local" | "plain-prefix")))
    {
        let url = case["url"].as_str().unwrap();
        let bytes = if case["kind"] == "json-local" {
            serde_json::json!({"url":url}).to_string().into_bytes()
        } else {
            url.as_bytes().to_vec()
        };
        assert_eq!(
            upload::request::output(&bytes, "", "", true).unwrap().url,
            url
        );
        assert!(case["closed"].as_bool().unwrap());
    }
    for case in fixture["multipart"].as_array().unwrap() {
        let env = case["fields"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, value)| {
                (
                    format!("MARK_SHOT_UPLOAD_FIELD_{name}"),
                    value.as_str().unwrap().to_owned(),
                )
            })
            .collect();
        let config = Upload {
            env,
            ..Default::default()
        };
        let bytes = upload::request::multipart(&config, &[1, 2, 3], "screenshot.png").unwrap();
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(hex, case["body"].as_str().unwrap());
        assert_eq!(
            upload::request::BOUNDARY,
            case["boundary"].as_str().unwrap()
        );
    }
}

/// 【上传测试】【响应路径】验证点分隔数组、原版常用图床回退路径和纯 URL 第一行，不接受虚假成功
#[test]
fn original_response_paths_plain_urls_and_invalid_shapes() {
    for (body, path, expected) in [
        (
            r#"{"data":[{"url":"https://example.test/array"}]}"#,
            "data.0.url",
            "https://example.test/array",
        ),
        (
            r#"{"data":{"url":"https://example.test/smms"}}"#,
            "",
            "https://example.test/smms",
        ),
        (
            r#"{"data":{"link":"https://example.test/imgurl"}}"#,
            "",
            "https://example.test/imgurl",
        ),
        (
            r#"{"image":{"url":"https://example.test/imgbb"}}"#,
            "",
            "https://example.test/imgbb",
        ),
        (
            " \n https://example.test/plain\nignored",
            "",
            "https://example.test/plain",
        ),
    ] {
        assert_eq!(
            upload::request::output(body.as_bytes(), path, "", false)
                .unwrap()
                .url,
            expected
        );
    }
    for bytes in [
        b"{\"url\":\"\"}".as_slice(),
        b"[]",
        b"{\"errors\":[\"private-token\"]}",
        b"error\nhttps://example.test/later",
        b"file:///tmp/upload",
    ] {
        let error = upload::request::output(bytes, "", "", false).unwrap_err();
        assert_eq!(error.kind, upload::ErrorKind::Protocol);
        assert!(!error.to_string().contains("private-token"));
    }
    assert!(
        upload::request::output(
            br#"{"data":{"url":"https://example.test/helper-only"}}"#,
            "",
            "",
            true
        )
        .is_err()
    );
}

/// 【上传测试】【字段验证】控制字符不能进入 multipart 字段或请求头，空图片与超限响应直接失败
#[test]
fn malformed_field_and_response_inputs_are_rejected() {
    let config = Upload::from_value(
        &serde_json::json!({"env":{"MARK_SHOT_UPLOAD_FIELD":"image\r\ninjected"}}),
    );
    assert!(upload::request::multipart(&config, &[1, 2, 3], "image.png").is_err());
    assert!(upload::request::multipart(&Upload::default(), &[], "image.png").is_err());
    assert!(upload::request::output(&vec![b'a'; upload::MAX_RESPONSE + 1], "", "", false).is_err());
}
