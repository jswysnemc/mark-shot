//! 【翻译集成】【公开服务】仅在显式执行时向哔哩哔哩发送固定测试句，验证应用完整翻译任务

use mark_shot::{
    config::Translation,
    translation::{Provider, request::Request, task::Task},
};
use std::{
    path::Path,
    time::{Duration, Instant},
};

/// 【翻译集成】【真实请求】无参数，使用默认哔哩哔哩模型翻译固定文本并保留分段身份，返回空值
#[test]
#[ignore = "requires explicit access to Bilibili public translation service"]
fn bilibili_translates_fixed_text_through_application_task() {
    let mut config = Translation::default();
    config.provider = "plugin:bilibili".into();
    if let Ok(model) = std::env::var("MARK_SHOT_TEST_BILIBILI_MODEL") {
        config
            .settings
            .insert("bilibili".into(), serde_json::json!({"model": model}));
    }
    let mut task = Task::start_request(
        &config,
        Request::text("今天辛苦了！\n\n明天见。"),
        "Japanese",
        Path::new("/unused/config"),
        || {},
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(65);
    loop {
        if let Some(result) = task.take_result() {
            let result = result.unwrap();
            assert_eq!(result.provider, Provider::Bilibili);
            assert_eq!(result.segments.len(), 2);
            assert_eq!(result.segments[0].id, 0);
            assert_eq!(result.segments[1].id, 2);
            assert_ne!(result.segments[0].text, "今天辛苦了！");
            assert_ne!(result.segments[1].text, "明天见。");
            assert!(!result.text().contains("<think>"));
            println!("{}", result.text());
            break;
        }
        assert!(Instant::now() < deadline, "public translation timed out");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// 【翻译集成】【技术文本】无参数，检查本地常用模型处理英文技术段落时是否返回完整译文
#[test]
#[ignore = "requires explicit access to Bilibili public translation service"]
fn bilibili_translates_technical_segments() {
    let mut config = Translation::default();
    config.provider = "plugin:bilibili".into();
    config.settings.insert(
        "bilibili".into(),
        serde_json::json!({"model":"Index-Translate-35B-A3B"}),
    );
    let cases = [
        "Environment\nHerdr Version: 0.9.5 (Client and Server)\nOperating system: NixOS 26.11, x86_64 Linux\nTerminal: Whip's xterm.js with Kitty graphics enabled.\nRelevant config: [terminal] kitty-graphics = true, set before starting the test server.",
        "The preview remains blank after enabling Kitty graphics in the terminal renderer.\nSteps to reproduce:\n1. Open a terminal and start the server.\n2. Run the image preview command.\nExpected result: The image should appear inline.\nActual result: Only an empty area is displayed.",
        "Return an object with the keys \"name\" and \"value\".\nThe syntax is {\"enabled\": true}.\nDo not overwrite the original configuration file.",
    ];
    for source in cases {
        let mut task = Task::start_request(
            &config,
            Request::text(source),
            "Simplified Chinese",
            Path::new("/unused/config"),
            || {},
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(65);
        loop {
            if let Some(result) = task.take_result() {
                let output = result
                    .unwrap_or_else(|error| panic!("technical translation failed: {error:?}"));
                assert_eq!(output.segments.len(), source.lines().count());
                assert!(
                    output
                        .segments
                        .iter()
                        .all(|segment| !segment.text.trim().is_empty())
                );
                println!("{}", output.text());
                break;
            }
            assert!(Instant::now() < deadline, "public translation timed out");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
