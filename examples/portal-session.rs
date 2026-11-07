//! 【持续捕获回归】【私有入口】私有Portal协议服务与真实PipeWire会话验证，不连接用户服务

#[path = "portal-session/probe.rs"]
#[cfg(target_os = "linux")]
mod probe;
#[path = "portal-session/server.rs"]
#[cfg(target_os = "linux")]
mod server;

/// 【持续捕获回归】【入口校验】按参数运行私有服务或验证进程，要求拥有独立运行时标记，返回执行结果
#[tokio::main(flavor = "current_thread")]
#[cfg(target_os = "linux")]
async fn main() -> mark_shot::AppResult {
    let root = std::path::PathBuf::from(std::env::var("XDG_RUNTIME_DIR")?);
    if !root.join("portal-test-runtime.json").is_file() {
        return Err("portal test requires an owned private runtime".into());
    }
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("server") => server::run(&args[1..]).await,
        Some("probe") => probe::run(&args[1..]).await,
        _ => Err("expected server or probe mode".into()),
    }
}

/// 【持续捕获回归】【平台范围】无参数，明确拒绝没有Portal与PipeWire的目标系统，返回平台错误
#[cfg(not(target_os = "linux"))]
fn main() -> mark_shot::AppResult {
    Err("portal session verification requires Linux".into())
}
