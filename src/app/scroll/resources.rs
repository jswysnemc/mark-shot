//! 【滚动截图】【旧资源释放】进入终端捕获会话后不再保留冻结截图、滤镜或持久绘制层

use crate::editor::EditorState;

/// 【滚动截图】【像素回收】按 state 释放旧截图和编辑缓存；index 的控制画布与字体保留，不再恢复旧编辑状态，无返回值
pub(super) fn release(state: &mut EditorState, index: usize) {
    // 1. 【滚动截图】【后台回收】先关闭原图分析借用，再释放冻结像素
    state.region_detection = Default::default();
    state.captures.clear();
    state.file_canvas = None;
    state.dimmed.clear();
    state.annotations_layer.clear();
    state.filters.images.clear();
    state.annotations.clear();
    state.pending = None;
    state.text.editors.clear();
    // 2. 【滚动截图】【其他输出】只保留当前控制画布，其他输出无需整屏像素
    for (monitor, canvas) in state.canvas.iter_mut().enumerate() {
        if monitor != index {
            *canvas = tiny_skia::Pixmap::new(1, 1).expect("one-pixel scroll placeholder");
        }
    }
    crate::utils::memory::release_unused();
}
