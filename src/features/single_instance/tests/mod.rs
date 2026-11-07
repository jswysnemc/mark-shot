//! 【单实例回归】【有限入口】真实文件锁、Unix 连接和线程上下文，不依赖用户运行目录

mod context;
mod session_path;
mod state;
mod transport;

use super::{
    request::{Plan, Request},
    session_path::SessionPath,
};
use std::{
    os::unix::fs::{FileTypeExt, PermissionsExt},
    path::Path,
    time::Instant,
};

/// 【单实例回归】【初始参数】无参数，返回未延迟的普通截图命令与对应计划
fn capture() -> (Request, Plan) {
    let request = Request {
        capture: true,
        ..Request::default()
    };
    let plan = request.plan(Instant::now()).unwrap();
    (request, plan)
}

/// 【单实例回归】【私有路径】将本次临时 root 设为私有，再按 id 返回当前用户下的独立服务路径，不读取环境变量
fn path(root: &Path, id: i32) -> SessionPath {
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700)).unwrap();
    SessionPath::new(root, id, nix::unistd::Uid::effective().as_raw()).unwrap()
}

/// 【单实例回归】【服务路径】按 path 返回是否存在真实套接字，缺失或读取错误返回否
fn socket_exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_socket())
}
