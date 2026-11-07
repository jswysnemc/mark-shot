//! 【Portal捕获测试】【来源绑定】授权来源必须完整且唯一对应选定输出

use super::*;

/// 【Portal捕获测试】【来源构造】以 node_id 和 position 创建无像素的来源描述，返回测试来源
fn source(node_id: u32, position: Option<(i32, i32)>) -> StreamInfo {
    StreamInfo {
        node_id,
        position,
        size: Some((1920, 1080)),
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

/// 【Portal捕获测试】【完整目标】缺失来源、重复来源位置和空响应不得默认为捕获成功，无参数
#[test]
fn incomplete_or_duplicate_source_coverage_is_rejected() {
    assert!(match_sources(vec![], &[(0, 0)]).is_err());
    assert!(match_sources(vec![source(7, Some((0, 0)))], &[(0, 0), (1920, 0)]).is_err());
    assert!(
        match_sources(
            vec![source(7, Some((0, 0))), source(9, Some((0, 0)))],
            &[(0, 0), (1920, 0)]
        )
        .is_err()
    );
}
