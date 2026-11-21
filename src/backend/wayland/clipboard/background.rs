//! 【Wayland剪贴板】【后台持有】截图返回并释放锁后继续提供选择，接管后自然退出

use std::cell::RefCell;
type Service = Box<dyn FnOnce() -> crate::AppResult>;
thread_local! { static PENDING: RefCell<Option<Service>> = const { RefCell::new(None) }; }

/// 【Wayland剪贴板】【服务交接】按 service 暂存本次窗口连接，无返回值；调用方先销毁全部窗口与图像
pub(crate) fn defer(service: impl FnOnce() -> crate::AppResult + 'static) {
    PENDING.with(|pending| pending.replace(Some(Box::new(service))));
}

/// 【Wayland剪贴板】【后台执行】无参数，在捕获运行时和单实例锁释放后执行服务，返回传输结果，输出管道及时获得 EOF
pub(crate) fn serve_pending() -> crate::AppResult {
    let Some(service) = PENDING.with(|pending| pending.take()) else {
        return Ok(());
    };
    // 1. 【Wayland剪贴板】【输出释放】保留有效标准描述符，但不再持有调用方管道，避免后续文件复用已关闭的描述符
    use std::io::Write;
    use std::os::fd::AsRawFd;
    std::io::stdout().flush()?;
    std::io::stderr().flush()?;
    let null = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")?;
    for descriptor in [1, 2] {
        if unsafe { nix::libc::dup2(null.as_raw_fd(), descriptor) } == -1 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    crate::utils::memory::release_unused();
    service()
}
