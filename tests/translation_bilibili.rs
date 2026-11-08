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
