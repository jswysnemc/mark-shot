//! 【单实例回归】【文件边界】只使用真实私有目录及普通占用文件，不跟随链接或接触其他用户资源

use super::super::session_path::{SessionPath, wayland_identity};
use std::{
    ffi::OsStr,
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};

/// 【单实例回归】【命名空间身份】无参数，验证不可见 PID 仍可创建路径，同一显示名保持稳定且不同显示名分离，无返回值
#[test]
fn invisible_wayland_peer_uses_stable_display_identity() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let uid = nix::unistd::Uid::effective().as_raw();
    let display = Some(OsStr::new("wayland-0"));
    let identity = wayland_identity(0, uid, display).unwrap();
    assert!(identity > 0);
    let path = SessionPath::new(root.path(), identity, uid).unwrap();
    let repeated =
        SessionPath::new(root.path(), wayland_identity(0, uid, display).unwrap(), uid).unwrap();
    let other = SessionPath::new(
        root.path(),
        wayland_identity(0, uid, Some(OsStr::new("wayland-1"))).unwrap(),
        uid,
    )
    .unwrap();
    assert_eq!(path.socket, repeated.socket);
    assert_ne!(path.socket, other.socket);
    assert!(path.lock_file().is_ok());
}

/// 【单实例回归】【可见进程身份】无参数，验证可见合成器继续使用原 PID，显示名不改变已有会话路径，无返回值
#[test]
fn visible_wayland_peer_preserves_compositor_pid() {
    let uid = nix::unistd::Uid::effective().as_raw();
    for display in [
        None,
        Some(OsStr::new("wayland-0")),
        Some(OsStr::new("wayland-1")),
    ] {
        assert_eq!(wayland_identity(42, uid, display).unwrap(), 42);
    }
}

/// 【单实例回归】【对端用户认证】无参数，验证可见和不可见 PID 均拒绝其他用户，同时拒绝负数 PID，无返回值
#[test]
fn wayland_peer_rejects_foreign_uid_and_invalid_pid() {
    let uid = nix::unistd::Uid::effective().as_raw();
    for pid in [0, 42] {
        let error = wayland_identity(pid, uid + 1, Some(OsStr::new("wayland-0"))).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Wayland compositor does not belong to the current user"
        );
    }
    assert!(wayland_identity(-1, uid, None).is_err());
}

/// 【单实例回归】【默认显示身份】无参数，验证缺失显示名沿用 Wayland 默认名称，无返回值
#[test]
fn missing_wayland_display_matches_default_name() {
    let uid = nix::unistd::Uid::effective().as_raw();
    assert_eq!(
        wayland_identity(0, uid, None).unwrap(),
        wayland_identity(0, uid, Some(OsStr::new("wayland-0"))).unwrap()
    );
}

/// 【单实例回归】【目录认证】公有目录、链接与错误用户均在创建服务前拒绝，无参数、无返回值
#[test]
fn runtime_requires_private_directory_and_current_owner() {
    let root = tempfile::tempdir().unwrap();
    let uid = nix::unistd::Uid::effective().as_raw();
    assert!(SessionPath::new(root.path(), 1, uid + 1).is_err());
    let linked = root.path().join("linked");
    symlink(root.path(), &linked).unwrap();
    assert!(SessionPath::new(&linked, 1, uid).is_err());
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(SessionPath::new(root.path(), 1, uid).is_err());
}

/// 【单实例回归】【锁文件链接】已有链接不能作为占用文件，外部目标内容保持原值，无参数、无返回值
#[test]
fn lock_file_rejects_symbolic_and_hard_links() {
    for hard in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let path = super::path(root.path(), 5);
        let target = root.path().join("target");
        fs::write(&target, "preserved").unwrap();
        let lock = path.socket.with_extension("lock");
        if hard {
            fs::hard_link(&target, &lock).unwrap();
        } else {
            symlink(&target, &lock).unwrap();
        }
        assert!(path.lock_file().is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "preserved");
    }
}

/// 【单实例回归】【套接字类型】已有普通文件不会被当作失效服务删除，无参数、无返回值
#[test]
fn socket_path_rejects_ordinary_files() {
    let root = tempfile::tempdir().unwrap();
    let path = super::path(root.path(), 6);
    fs::write(&path.socket, "preserved").unwrap();
    assert!(path.socket_identity().is_err());
    path.remove_socket((0, 0));
    assert_eq!(fs::read_to_string(&path.socket).unwrap(), "preserved");
}
