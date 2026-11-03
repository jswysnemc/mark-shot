use std::sync::OnceLock;

/// 【文字识别】【并行调度】首次使用时限制图像处理线程池；普通截图不会创建这些线程
pub(super) fn initialize(threads: usize) {
    static INITIALIZED: OnceLock<()> = OnceLock::new();
    INITIALIZED.get_or_init(|| {
        if let Err(error) = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|index| format!("mark-ocr-{index}"))
            .build_global()
        {
            log::info!("【文字识别】【并行调度】线程池已由其他模块初始化: {error}");
        }
    });
}
