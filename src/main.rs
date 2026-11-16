#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

/// 【应用】【启动】读取系统命令行并执行应用入口；失败时返回非零退出码
fn main() {
    let mut arguments: Vec<_> = std::env::args_os().skip(1).collect();
    // 1. 【Windows启动】【默认后台】双击启动托盘服务，显式截图和内部任务保持原有入口
    if cfg!(target_os = "windows") && arguments.is_empty() {
        arguments.push("--background".into());
    }
    if let Err(error) = mark_shot::cli::run(arguments) {
        eprintln!("mark-shot: {error}");
        std::process::exit(1);
    }
}
