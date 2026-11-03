/// 【应用】【启动】读取系统命令行并执行应用入口；失败时返回非零退出码
fn main() {
    if let Err(error) = mark_shot::cli::run(std::env::args_os().skip(1)) {
        eprintln!("mark-shot: {error}");
        std::process::exit(1);
    }
}
