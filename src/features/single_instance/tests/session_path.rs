//! 【单实例回归】【文件边界】只使用真实私有目录及普通占用文件，不跟随链接或接触其他用户资源

use super::super::session_path::SessionPath;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};

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
