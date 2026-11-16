//! 【开机启动回归】【事务边界】使用替代注册器验证失败路径，不修改测试机器的启动设置

use super::transaction;
use std::{cell::Cell, io};

/// 【开机启动回归】【注册失败】无参数，系统拒绝注册时配置写入不执行
#[test]
fn registration_failure_does_not_save_configuration() {
    let saved = Cell::new(false);
    let result = transaction(
        true,
        false,
        |_| Err(io::Error::other("access denied")),
        || {
            saved.set(true);
            Ok(())
        },
    );
    assert!(result.is_err());
    assert!(!saved.get());
}

/// 【开机启动回归】【保存失败】无参数，配置保存失败会恢复原启动开关，返回原始保存错误
#[test]
fn save_failure_restores_previous_registration() {
    for previous in [false, true] {
        let mut states = Vec::new();
        let result = transaction(
            !previous,
            previous,
            |enabled| {
                states.push(enabled);
                Ok(())
            },
            || Err(io::Error::other("configuration is read-only")),
        );
        assert_eq!(states, [!previous, previous]);
        assert_eq!(
            result.unwrap_err().to_string(),
            "configuration is read-only"
        );
    }
}

/// 【开机启动回归】【恢复失败】无参数，恢复注册失败时返回保存及恢复两项原因，避免报告保存成功
#[test]
fn rollback_failure_reports_both_errors() {
    let result = transaction(
        true,
        false,
        |enabled| {
            if enabled {
                Ok(())
            } else {
                Err(io::Error::other("rollback denied"))
            }
        },
        || Err(io::Error::other("save denied")),
    );
    let message = result.unwrap_err().to_string();
    assert!(message.contains("save denied") && message.contains("rollback denied"));
}
