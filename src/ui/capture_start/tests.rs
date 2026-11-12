//! 【截图起始测试】【尺寸约束】低分辨率和分数缩放共享逻辑尺寸，不占用大面积画布

use super::{Control, State, layout, text};
use crate::features::recording::Status;

/// 【截图起始测试】【提示完整性】常见低分辨率必须同时显示两组各七条提示，不能把录制快捷键裁掉
#[test]
fn expanded_hints_remain_visible_on_common_laptop_outputs() {
    for size in [
        (853, 480),
        (853, 533),
        (1024, 640),
        (1270, 720),
        (1366, 768),
        (1920, 1080),
    ] {
        let grid = layout::hint_grid(size, (7, 7)).unwrap();
        assert!(grid.columns * grid.rows >= 14, "{size:?}");
    }
}

/// 【截图录制测试】【编辑阶段控制】起始提示隐藏后只显示录制控制，空闲会话和文件编辑不保留无效控制栏
#[test]
fn compact_recording_bar_survives_intro_without_reintroducing_capture_entries() {
    let mut state = State {
        recording_controls: true,
        ..Default::default()
    };
    state.status.phase = "recording".into();
    assert!(state.shows_bar() && state.polls_recording());
    for size in [(1270, 720), (640, 360), (320, 240)] {
        let controls = layout::toolbar_controls(size, &state, 0);
        assert_eq!(
            controls
                .iter()
                .map(|(control, _)| *control)
                .collect::<Vec<_>>(),
            [Control::Pause, Control::Stop]
        );
        let status = layout::toolbar_status(size, &state, 0);
        assert!(status.right() < controls[0].1.left());
        assert!(status.width() >= 60.0);
        for (control, bounds) in controls {
            assert_eq!(
                layout::toolbar_hit(
                    size,
                    &state,
                    0,
                    (
                        f64::from(bounds.left() + bounds.width() / 2.0),
                        f64::from(bounds.top() + bounds.height() / 2.0)
                    )
                ),
                Some(control)
            );
        }
    }
    state.status.phase = "idle".into();
    assert!(!state.shows_bar());
    state.recording_controls = false;
    assert!(!state.polls_recording());
}

/// 【截图录制测试】【倒计时取消】准备阶段明确显示取消，兼容旧状态来源没有格式字段的情况
#[test]
fn countdown_controls_cancel_and_preserve_legacy_status_decoding() {
    let mut status: Status =
        serde_json::from_str(r#"{"phase":"countdown","elapsed_ms":0,"detail":"3","output":null}"#)
            .unwrap();
    status.format = "gif".into();
    assert_eq!(text::status(&status, true), "GIF · 3 秒后开始");
    assert_eq!(
        text::control_label(Control::Stop, &status, true),
        "取消录制"
    );
    status.phase = "recording".into();
    assert_eq!(
        text::control_label(Control::Stop, &status, true),
        "停止并保存"
    );
}

/// 【截图起始测试】【重复停止】已发送停止后仍收到旧录制快照也不允许再次停止，保存阶段不接受取消导出
#[test]
fn stop_is_claimed_once_until_the_session_ends() {
    let mut state = State::default();
    state.status.phase = "recording".into();
    assert!(state.request_stop());
    assert!(!state.request_stop());
    state.status.phase = "paused".into();
    assert!(!state.request_stop());
    state.status.phase = "saving".into();
    assert!(!state.request_stop());
}

/// 【截图起始测试】【活动可用性】暂停只在录制阶段有效，停止在导出前有效且请求一次后锁定
#[test]
fn control_availability_follows_the_recording_phase() {
    let mut state = State::default();
    for phase in ["idle", "countdown", "starting", "saving", "finished"] {
        state.status.phase = phase.into();
        assert!(!state.can_pause(), "{phase}");
    }
    for phase in ["idle", "saving", "finished", "failed"] {
        state.status.phase = phase.into();
        assert!(!state.can_stop(), "{phase}");
    }
    for phase in ["countdown", "starting", "recording", "paused"] {
        state.status.phase = phase.into();
        assert!(state.can_stop(), "{phase}");
    }
    state.status.phase = "recording".into();
    assert!(state.can_pause());
    state.stop_requested = true;
    assert!(!state.can_pause() && !state.can_stop());
}

/// 【截图起始测试】【入口与消息】已有录制窗口时禁用录制入口，消息条只在没有活动录制时显示
#[test]
fn entry_and_message_visibility_follow_the_session() {
    let mut state = State::default();
    assert!(!state.can_record() && !state.message_visible());
    state.checked = true;
    assert!(state.can_record());
    state.status.window_open = true;
    assert!(!state.can_record() && state.message_visible());
    state.status.phase = "recording".into();
    assert!(!state.message_visible());
}

/// 【截图起始测试】【低分辨率】提示保留完整键位列宽且不超出紧凑范围，入口始终位于屏幕内且不相交
#[test]
fn compact_panels_fit_low_resolution_and_scaled_outputs() {
    for size in [(1920, 1080), (1270, 720), (853, 480), (640, 360)] {
        let hints = layout::hints(size);
        assert!(hints.width() <= size.0 as f32 / if size.0 < 1280 { 2.5 } else { 4.0 });
        assert!(hints.height() <= size.1 as f32 / 4.0);
        assert!(hints.bottom() <= size.1 as f32 && hints.right() <= size.0 as f32);
        for active in [false, true] {
            let controls = layout::controls(size, active);
            for (control, bounds) in &controls {
                assert!(bounds.left() >= 0.0 && bounds.right() <= size.0 as f32);
                assert!(bounds.top() >= 0.0 && bounds.bottom() <= size.1 as f32);
                let center = (
                    (bounds.left() + bounds.width() / 2.0) as f64,
                    (bounds.top() + bounds.height() / 2.0) as f64,
                );
                assert_eq!(layout::hit(size, active, center), Some(*control));
            }
            for pair in controls.windows(2) {
                assert!(pair[0].1.right() < pair[1].1.left());
            }
        }
        assert_eq!(
            layout::hit(
                size,
                false,
                (hints.left() as f64 + 2.0, hints.top() as f64 + 2.0)
            ),
            None
        );
        let active = layout::controls(size, true);
        assert_eq!(
            active
                .iter()
                .filter(|(control, _)| *control == Control::Pause)
                .count(),
            1
        );
        assert_eq!(
            active
                .iter()
                .filter(|(control, _)| matches!(control, Control::Video | Control::Gif))
                .count(),
            0
        );
    }
}

/// 【截图起始测试】【状态区域】活动布局把状态文字放在截图入口与暂停之间，留出可见宽度
#[test]
fn status_area_sits_between_entries_and_controls() {
    for size in [(1920, 1080), (1270, 720), (640, 360)] {
        let items = layout::controls(size, true);
        let status = layout::status(size);
        assert!(status.left() >= items[1].1.right());
        assert!(status.right() <= items[2].1.left());
        assert!(status.width() >= 60.0);
    }
}

/// 【截图起始测试】【提示网格】不同分辨率下网格不超出最大范围，行数不超过可用空间
#[test]
fn hint_grid_never_exceeds_the_reserved_panel() {
    for size in [(1920, 1080), (1366, 768), (853, 480), (640, 360)] {
        let limit = layout::hints(size);
        for counts in [(6, 6), (1, 0), (12, 12)] {
            let Some(grid) = layout::hint_grid(size, counts) else {
                continue;
            };
            assert!(grid.bounds.width() <= limit.width());
            assert!(grid.bounds.height() <= limit.height());
            assert!(grid.bounds.bottom() <= limit.bottom() && grid.bounds.right() <= limit.right());
            let fit =
                ((limit.height() - grid.padding * 2.0) / grid.row_height + 0.001).max(0.0) as usize;
            assert!(grid.rows <= fit && grid.rows > 0);
            assert!((16.0..=layout::HINT_ROW).contains(&grid.row_height));
            for column in 0..grid.columns {
                let cell = layout::hint_cell(&grid, column, 0);
                assert!(cell.right() <= grid.bounds.right());
                assert!(cell.left() >= grid.bounds.left());
            }
        }
    }
}

/// 【截图起始测试】【状态文案】录制类型、阶段与倒计时使用同一摘要格式，消息条区分保存与失败
#[test]
fn status_and_message_texts_cover_every_phase() {
    let mut status = Status {
        window_open: true,
        phase: "recording".into(),
        elapsed_ms: 72_000,
        output: Some("/tmp/Recording.mp4".into()),
        ..Default::default()
    };
    assert_eq!(text::status(&status, true), "录屏 · 录制中 01:12");
    status.phase = "paused".into();
    assert_eq!(text::status(&status, false), "Video · Paused 01:12");
    status.phase = "countdown".into();
    status.detail = "3".into();
    assert_eq!(text::status(&status, true), "录屏 · 3 秒后开始");
    status.output = Some("/tmp/Recording.gif".into());
    assert_eq!(text::status(&status, false), "GIF · starts in 3s");
    status.phase = "finished".into();
    assert_eq!(text::message(&status, "", true), "录制已保存 Recording.gif");
    status.phase = "failed".into();
    assert_eq!(
        text::message(&status, "", false),
        "Recording failed. See the recording window"
    );
    status.phase = "idle".into();
    assert_eq!(
        text::message(&status, "", true),
        "录制窗口已打开，请在原窗口操作"
    );
    status.window_open = false;
    assert_eq!(text::message(&status, "稍后重试", true), "稍后重试");
}
