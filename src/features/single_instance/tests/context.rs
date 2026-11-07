//! 【单实例回归】【范围恢复】辅助窗口只在本次截图范围内继承控制句柄

use super::super::{Entry, Owner, context};
use std::sync::Arc;

/// 【单实例回归】【嵌套上下文】不同范围进入与退出后恢复原句柄，退出不会保留当前线程的隐式占用，无参数、无返回值
#[tokio::test(flavor = "current_thread")]
async fn nested_scopes_restore_previous_capture_controller() {
    let root = tempfile::tempdir().unwrap();
    let (request, plan) = super::capture();
    let Entry::Owner(first) = Owner::start_at(super::path(root.path(), 1), request.clone(), plan)
        .await
        .unwrap()
    else {
        panic!("expected first owner");
    };
    let Entry::Owner(second) = Owner::start_at(super::path(root.path(), 2), request, plan)
        .await
        .unwrap()
    else {
        panic!("expected second owner");
    };
    assert!(context::current().is_none());
    let first_scope = context::enter(&first.controller);
    assert!(Arc::ptr_eq(&context::current().unwrap(), &first.controller));
    let second_scope = context::enter(&second.controller);
    assert!(Arc::ptr_eq(
        &context::current().unwrap(),
        &second.controller
    ));
    drop(second_scope);
    assert!(Arc::ptr_eq(&context::current().unwrap(), &first.controller));
    drop(first_scope);
    assert!(context::current().is_none());
}
