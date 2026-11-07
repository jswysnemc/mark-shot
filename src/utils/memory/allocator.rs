//! 【内存生命周期】【分配策略】隔离平台分配器设置，供工作线程创建前的启动入口使用

/// 【内存生命周期】【平台初始化】无参数，GNU Linux固定大对象映射阈值，其他平台使用原生分配器，无返回值
pub(super) fn configure() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        const LARGE_BYTES: nix::libc::c_int = 4 * 1024 * 1024;
        static INITIALIZED: std::sync::Once = std::sync::Once::new();
        INITIALIZED.call_once(|| {
            // 1. 【内存生命周期】【大对象映射】禁用动态提高阈值，捕获帧释放时直接归还映射，避免留在线程内存区
            let configured =
                unsafe { nix::libc::mallopt(nix::libc::M_MMAP_THRESHOLD, LARGE_BYTES) };
            if configured == 0 {
                log::error!("【内存生命周期】【大对象映射】无法初始化分配阈值");
            }
        });
    }
}
