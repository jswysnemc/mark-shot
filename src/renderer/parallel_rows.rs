//! 【画布绘制】【有界并行】采样与遮罩共用固定线程池，按互不重叠的完整行分配写入

use rayon::prelude::*;
use std::sync::OnceLock;

/// 【画布绘制】【行带分配】按 stride 和 rows 切分 pixels，draw 接收首行索引及独占字节；小区域或线程创建失败时顺序处理，无返回值
pub(super) fn for_each(
    pixels: &mut [u8],
    stride: usize,
    rows: usize,
    draw: impl Fn(usize, &mut [u8]) + Sync + Send,
) {
    static POOL: OnceLock<Option<rayon::ThreadPool>> = OnceLock::new();
    let pool = POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(
                std::thread::available_parallelism()
                    .map_or(1, usize::from)
                    .min(4),
            )
            .thread_name(|index| format!("canvas-render-{index}"))
            .build()
            .ok()
    });
    let bytes = stride * rows;
    if pixels.len() >= 256 * 256 * 4
        && let Some(pool) = pool
    {
        pool.install(|| {
            pixels
                .par_chunks_mut(bytes)
                .enumerate()
                .for_each(|(index, band)| draw(index * rows, band));
        });
    } else {
        for (index, band) in pixels.chunks_mut(bytes).enumerate() {
            draw(index * rows, band);
        }
    }
}
