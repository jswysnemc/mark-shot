//! 【截图初始化】【输出画布】只分配最终输出画布，旧缓存接口使用小型占位缓冲

use crate::{renderer, types::Placement};
use tiny_skia::Pixmap;

/// 【截图初始化】【前景图层】按placements和transparent返回三组图层，全屏原生模式保持尚未绘制的匿名零页
pub(super) fn build_with_transparency(
    placements: &[Placement],
    transparent: bool,
) -> (Vec<Pixmap>, Vec<Pixmap>, Vec<Pixmap>) {
    let mut canvas = Vec::with_capacity(placements.len());
    let mut dimmed = Vec::with_capacity(placements.len());
    let mut annotations = Vec::with_capacity(placements.len());
    for placement in placements {
        let (width, height) = placement.px_size();
        let mut output = Pixmap::new(width, height).expect("cannot allocate output canvas");
        crate::utils::transparent_pixmap::prepare_sparse(&mut output);
        #[cfg(target_os = "linux")]
        if transparent {
            crate::utils::transparent_pixmap::clear(&mut output);
        }
        if !transparent {
            renderer::init_dimming(&mut output, None, None, placement.scale);
        }
        canvas.push(output);
        dimmed.push(Pixmap::new(1, 1).expect("cannot allocate dimming placeholder"));
        annotations.push(Pixmap::new(1, 1).expect("cannot allocate annotation placeholder"));
    }
    (canvas, dimmed, annotations)
}
