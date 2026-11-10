//! 【翻译测试回归】【真实请求】覆盖设置事件、译文和错误显示，以及取消和切换服务后的结果隔离

use super::{test_server::Server, *};
use crate::{
    backend::window::{LEFT_BUTTON, WindowContent, WindowEvent},
    features::preferences::{
        catalog::Page,
        translation::{
            fields::Field,
            layout::{Hit, Layout},
            state::State,
        },
        window::Preferences,
    },
};
use serde_json::json;
use std::time::Duration;

/// 【翻译测试回归】【窗口构造】按 server 创建临时配置及设置窗口，显式虚构密钥避免读取真实凭据
fn preferences(server: &Server) -> Preferences {
    let mut state = Preferences::new(false).unwrap();
    state.page = Page::Translation;
    state.values["translation"] = json!({"provider":"plugin:openai-responses","target_language":"Simplified Chinese","responses":{"baseUrl":server.endpoint,"apiKey":"preview-private-key","model":"preview-model","timeoutMs":1000}});
    state.translation = State::new(&state.values);
    state
}

/// 【翻译测试回归】【点击测试】按 state 通过窗口按下与释放事件执行测试或取消，不直接调用任务实现
fn click_test(state: &mut Preferences) {
    let layout = Layout::new(state);
    let (_, bounds) = layout
        .controls
        .iter()
        .find(|(hit, _)| *hit == Hit::Test)
        .unwrap();
    let position = (
        (bounds.left() + bounds.width() / 2.0) as f64,
        (bounds.top() + 10.0) as f64,
    );
    assert_eq!(layout.hit(position), Some(Hit::Test));
    for pressed in [true, false] {
        state.event(WindowEvent::PointerButton {
            position,
            button: LEFT_BUTTON,
            pressed,
        });
    }
}

/// 【翻译测试回归】【结果唤醒】按 state 驱动真实完成事件，超时使测试失败，不启动额外请求
fn finish(state: &mut Preferences) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while state.translation.preview.busy() && Instant::now() < deadline {
        state.event(WindowEvent::Wake);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!state.translation.preview.busy());
}

/// 【翻译测试回归】【服务一致】无参数，使用当前查看的模型和预制原文，成功显示真实译文且不改变已启用配置
#[test]
fn preview_uses_selected_service_and_renders_real_translation() {
    let content = json!({"translations":[{"id":0,"text":"你好！这是一段翻译测试。"},{"id":1,"text":"会议将在明天上午 9:30 开始。"}]}).to_string();
    let server = Server::start(Some((
        200,
        json!({"status":"completed","error":null,"output":[{"type":"message","content":[{"type":"output_text","text":content}]}]})
            .to_string(),
    )));
    let mut state = preferences(&server);
    state.translation.root["provider"] = "plugin:bilibili".into();
    state.translation.root["command"] = "must-not-run".into();
    click_test(&mut state);
    let request = server.seen.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(request.starts_with("POST /v1/responses HTTP/1.1"));
    assert!(request.contains("preview-model") && request.contains("9:30"));
    finish(&mut state);
    assert!(matches!(
        state.translation.preview.result,
        ResultState::Success(_)
    ));
    assert!(
        state
            .translation
            .preview
            .text(true)
            .contains("明天上午 9:30")
    );
    assert_eq!(state.translation.root["provider"], "plugin:bilibili");
    assert!(!state.path.exists());
    let mut image = tiny_skia::Pixmap::new(state.size.0, state.size.1).unwrap();
    state.draw(&mut image, 1.0);
    assert!(state.translation.preview.copyable());
}

/// 【翻译测试回归】【错误可见】无参数，HTTP 鉴权、无效响应和空结果都保留可复制错误详情，上游回显密钥必须脱敏
#[test]
fn preview_displays_http_and_protocol_errors_without_credentials() {
    for (status, body, expected) in [
        (
            401,
            r#"{"error":{"message":"invalid preview-private-key"}}"#,
            ErrorKind::Http,
        ),
        (200, "not JSON", ErrorKind::Protocol),
        (200, r#"{"output":[]}"#, ErrorKind::Protocol),
    ] {
        let server = Server::start(Some((status, body.into())));
        let mut state = preferences(&server);
        click_test(&mut state);
        finish(&mut state);
        let ResultState::Failed(error) = &state.translation.preview.result else {
            panic!("failure was hidden")
        };
        assert_eq!(error.kind, expected);
        let text = state.translation.preview.text(true);
        assert!(!text.contains("preview-private-key"));
        if status == 401 {
            assert!(text.contains("401") && text.contains("[redacted]"));
        }
        assert!(state.translation.preview.copyable());
        let mut image = tiny_skia::Pixmap::new(state.size.0, state.size.1).unwrap();
        state.draw(&mut image, 1.0);
        let bounds = Layout::new(&state).preview.result;
        let error_pixels = ((bounds.top() as u32 + 8)..(bounds.bottom() as u32 - 8))
            .flat_map(|y| {
                ((bounds.left() as u32 + 8)..(bounds.right() as u32 - 20)).map(move |x| (x, y))
            })
            .filter(|&(x, y)| {
                let pixel = image.pixel(x, y).unwrap().demultiply();
                pixel.red() > pixel.green().saturating_add(40)
                    && pixel.red() > pixel.blue().saturating_add(40)
            })
            .count();
        assert!(
            error_pixels > 20,
            "error details were not painted inside the preview"
        );
    }
}

/// 【翻译测试回归】【超时取消】无参数，超时显示明确错误；用户取消和服务切换立即关闭连接，晚到事件不能覆盖状态
#[test]
fn timeout_cancel_and_provider_change_do_not_leave_stale_results() {
    for mode in [0, 1, 2] {
        let server = Server::start(None);
        let mut state = preferences(&server);
        click_test(&mut state);
        server.seen.recv_timeout(Duration::from_secs(2)).unwrap();
        match mode {
            0 => {
                finish(&mut state);
                assert!(matches!(
                    state.translation.preview.result,
                    ResultState::Failed(Error {
                        kind: ErrorKind::Timeout,
                        ..
                    })
                ));
            }
            1 => {
                click_test(&mut state);
                assert!(matches!(
                    state.translation.preview.result,
                    ResultState::Canceled
                ));
            }
            _ => {
                super::super::input::choose(&mut state, Field::Provider, "0");
                assert!(matches!(
                    state.translation.preview.result,
                    ResultState::Idle
                ));
            }
        }
        server.closed.recv_timeout(Duration::from_secs(2)).unwrap();
        state.event(WindowEvent::Wake);
        assert!(!state.translation.preview.busy());
    }
}

/// 【翻译测试回归】【长错误布局】无参数，窄窗口中长错误按字形换行并独立滚动，滚轮不移动外部设置列表
#[test]
fn long_errors_wrap_and_scroll_within_preview() {
    let mut state = Preferences::new(false).unwrap();
    state.page = Page::Translation;
    state.size = (580, 650);
    state
        .translation
        .preview
        .fail(Error::new(ErrorKind::Api, "very_long_error_".repeat(100)));
    let mut image = tiny_skia::Pixmap::new(580, 650).unwrap();
    state.draw(&mut image, 1.0);
    let layout = Layout::new(&state);
    assert!(
        layout
            .preview
            .scrollbar(&state.translation.preview)
            .is_some()
    );
    let before = state.translation.scroll;
    state.event(WindowEvent::Scroll {
        position: (
            (layout.preview.result.left() + 10.0) as f64,
            (layout.preview.result.top() + 10.0) as f64,
        ),
        delta: (0.0, 1.0),
    });
    assert!(state.translation.preview.scroll > 0);
    assert_eq!(state.translation.scroll, before);
}
