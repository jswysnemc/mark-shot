//! 【Portal捕获测试】【来源绑定】授权来源必须完整且唯一对应选定输出，失效恢复令牌与热插拔布局需按身份恢复

use super::*;
use crate::types::linux_output::Info;
use std::sync::Mutex;

/// 【Portal捕获测试】【来源构造】以 node_id 和 position 创建无像素的来源描述，返回测试来源
fn source(node_id: u32, position: Option<(i32, i32)>) -> StreamInfo {
    StreamInfo {
        node_id,
        position,
        size: Some((1920, 1080)),
    }
}

/// 【Portal捕获测试】【输出构造】以 id、name、position 和 size 构造测试显示器描述，返回无协议句柄的 Output
fn output(id: u32, name: &str, position: (i32, i32), size: (i32, i32)) -> Output {
    Output {
        wl_output: None,
        info: Info::x11(id, name.into(), position, size),
    }
}

struct FakeAuthorizer {
    calls: Mutex<Vec<Option<String>>>,
    responses: Mutex<Vec<std::result::Result<Authorized<u32>, ashpd::Error>>>,
    discarded: Mutex<Vec<u32>>,
    forgotten: Mutex<usize>,
    refreshed: Mutex<Option<Vec<Output>>>,
}

#[async_trait]
impl Authorizer for FakeAuthorizer {
    type Session = u32;

    /// 【Portal捕获测试】【模拟启动】记录传入 token 并返回预设授权响应
    async fn start(
        &self,
        token: Option<&str>,
    ) -> std::result::Result<Authorized<Self::Session>, ashpd::Error> {
        self.calls.lock().unwrap().push(token.map(str::to_owned));
        self.responses.lock().unwrap().remove(0)
    }

    /// 【Portal捕获测试】【模拟回收】记录被关闭的会话编号 handle，无返回值
    async fn discard(&self, handle: Self::Session) {
        self.discarded.lock().unwrap().push(handle);
    }

    /// 【Portal捕获测试】【模拟清令牌】累加清除恢复令牌的调用次数，无返回值
    fn forget_token(&self) {
        *self.forgotten.lock().unwrap() += 1;
    }

    /// 【Portal捕获测试】【模拟刷新】返回预设的热插拔更新输出列表并按身份对齐 selected
    fn refresh_outputs(&self, selected: &[Output]) -> Option<Vec<Output>> {
        let latest = self.refreshed.lock().unwrap().clone()?;
        merge_refreshed_outputs(selected, &latest)
    }
}

/// 【Portal捕获测试】【排序与筛选】来源排序按目标输出且忽略未选输出，不能使用响应顺序代替真实位置，无参数
#[test]
fn source_identity_survives_reordered_and_extra_portal_streams() {
    let matched = match_sources(
        vec![
            source(9, Some((1920, 0))),
            source(7, Some((0, 0))),
            source(4, Some((3840, 0))),
        ],
        &[(0, 0), (1920, 0)],
    )
    .unwrap();
    assert_eq!(
        matched
            .iter()
            .map(|(index, stream)| (*index, stream.node_id))
            .collect::<Vec<_>>(),
        [(0, 7), (1, 9)]
    );
}

/// 【Portal捕获测试】【缺失位置】无位置只接受唯一来源和唯一目标，多来源或多目标均明确失败，无参数
#[test]
fn missing_positions_cannot_guess_multiple_output_identity() {
    assert!(match_sources(vec![source(7, None)], &[(1920, 0)]).is_ok());
    assert!(match_sources(vec![source(7, None), source(9, None)], &[(0, 0), (1920, 0)]).is_err());
    assert!(match_sources(vec![source(7, None), source(9, Some((0, 0)))], &[(1920, 0)]).is_err());
}

/// 【Portal捕获测试】【完整目标】缺失来源、重复来源位置和错屏单流均明确失败，不盲目按数量硬绑，无参数
#[test]
fn incomplete_duplicate_or_mismatched_source_coverage_is_rejected() {
    assert!(match_sources(vec![], &[(0, 0)]).is_err());
    assert!(match_sources(vec![source(7, Some((0, 0)))], &[(0, 0), (1920, 0)]).is_err());
    assert!(match_sources(vec![source(7, Some((0, 1080)))], &[(1920, 0)]).is_err());
    assert!(
        match_sources(
            vec![source(7, Some((0, 0))), source(9, Some((0, 0)))],
            &[(0, 0), (1920, 0)]
        )
        .is_err()
    );
}

/// 【Portal捕获测试】【令牌失效重试】连接第二块显示器后旧恢复令牌仅返回单屏流，必须清理令牌并无令牌重试一次
#[test]
fn saved_token_covering_fewer_monitors_is_forgotten_and_retried_once() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let authorizer = FakeAuthorizer {
        calls: Mutex::new(Vec::new()),
        responses: Mutex::new(vec![
            Ok(Authorized {
                session: 1,
                streams: vec![source(7, Some((0, 0)))],
                restore_token: Some("stale-token".into()),
            }),
            Ok(Authorized {
                session: 2,
                streams: vec![source(7, Some((0, 0))), source(9, Some((1920, 0)))],
                restore_token: Some("fresh-token".into()),
            }),
        ]),
        discarded: Mutex::new(Vec::new()),
        forgotten: Mutex::new(0),
        refreshed: Mutex::new(None),
    };
    let outputs = [
        output(10, "eDP-1", (0, 0), (1920, 1080)),
        output(11, "HDMI-A-1", (1920, 0), (1920, 1080)),
    ];
    let (session, matched, token) = runtime
        .block_on(negotiate(&authorizer, Some("saved-token"), &outputs))
        .unwrap();
    assert_eq!(session, 2);
    assert_eq!(token.as_deref(), Some("fresh-token"));
    assert_eq!(*authorizer.discarded.lock().unwrap(), vec![1]);
    assert_eq!(*authorizer.forgotten.lock().unwrap(), 1);
    assert_eq!(
        *authorizer.calls.lock().unwrap(),
        vec![Some("saved-token".into()), None]
    );
    assert_eq!(
        matched
            .iter()
            .map(|(index, info)| (*index, info.node_id))
            .collect::<Vec<_>>(),
        [(0, 7), (1, 9)]
    );
}

/// 【Portal捕获测试】【热插拔布局同步】授权窗口期间显示器位置完成迁移时，刷新输出后直接精确匹配且不误删有效令牌
#[test]
fn hotplug_position_settlement_refreshes_outputs_without_forgetting_valid_token() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let authorizer = FakeAuthorizer {
        calls: Mutex::new(Vec::new()),
        responses: Mutex::new(vec![Ok(Authorized {
            session: 5,
            streams: vec![source(12, Some((1920, 0)))],
            restore_token: Some("valid-token".into()),
        })]),
        discarded: Mutex::new(Vec::new()),
        forgotten: Mutex::new(0),
        refreshed: Mutex::new(Some(vec![
            output(11, "HDMI-A-1", (0, 0), (1920, 1080)),
            output(10, "eDP-1", (1920, 0), (1920, 1080)),
        ])),
    };
    let stale = [output(10, "eDP-1", (0, 0), (1920, 1080))];
    let (session, matched, token) = runtime
        .block_on(negotiate(&authorizer, Some("valid-token"), &stale))
        .unwrap();
    assert_eq!(session, 5);
    assert_eq!(token.as_deref(), Some("valid-token"));
    assert!(authorizer.discarded.lock().unwrap().is_empty());
    assert_eq!(*authorizer.forgotten.lock().unwrap(), 0);
    assert_eq!(matched[0].0, 0);
    assert_eq!(matched[0].1.position, Some((1920, 0)));
}

/// 【Portal捕获测试】【部分输出容错】多屏中个别副屏超时未交付帧时保留已捕获主屏，全部输出失败时返回首个错误，无参数
#[test]
fn partial_output_timeout_keeps_captured_monitors_and_all_failed_errors() {
    use crate::backend::wayland::capture::stream::PipewireFrame;

    let ok_frame = PipewireFrame {
        pixels: vec![255; 16],
        width: 2,
        height: 2,
        stride: 8,
    };
    let frames = collect_frames(vec![
        (0, source(71, Some((0, 0))), Ok(ok_frame)),
        (
            1,
            source(78, Some((3840, 0))),
            Err("PipeWire did not deliver a frame in time".into()),
        ),
    ])
    .unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].output, 0);
    assert_eq!(frames[0].info.node_id, 71);

    let Err(error) = collect_frames(vec![
        (
            0,
            source(71, Some((0, 0))),
            Err("PipeWire did not deliver a frame in time".into()),
        ),
        (
            1,
            source(78, Some((3840, 0))),
            Err("secondary error".into()),
        ),
    ]) else {
        panic!("all-failed outputs must return an error");
    };
    assert!(
        error
            .to_string()
            .contains("PipeWire did not deliver a frame in time")
    );
}
