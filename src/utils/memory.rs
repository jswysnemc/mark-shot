//! 【内存生命周期】【空闲页】在大图资源交接边界回收分配器空闲页，不改变仍在使用的像素

mod allocator;

#[cfg(test)]
mod tests;

/// 【内存生命周期】【启动策略】无参数，初始化当前进程的大对象分配策略，无返回值，不修改系统配置
pub(crate) fn configure() {
    allocator::configure();
}

/// 【内存生命周期】【页面回收】无参数、无返回值；GNU Linux 回收空闲堆页，其他平台由原生分配器管理
pub(crate) fn release_unused() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    // 1. 【内存生命周期】【分配器边界】malloc_trim 自行同步所有堆，只处理已释放区间，无外部指针参数
    unsafe {
        nix::libc::malloc_trim(0);
    }
}
