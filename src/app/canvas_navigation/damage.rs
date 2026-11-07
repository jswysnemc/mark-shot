//! 【画布导航】【损坏映射】画布边界使用共享变换，屏幕控件保留输出局部坐标

use crate::editor::{EditorState, dirty::mark_all_dirty};

/// 【画布导航】【视图损坏】按state登记每个输出完整屏幕及控件位置，dirty返回重绘掩码，无返回值
pub(crate) fn changed(state: &mut EditorState, dirty: &mut u32) {
    for monitor in 0..state.placements.len() {
        if let Some(rect) = state.placements[monitor].rect() {
            state.damage_local(monitor, rect);
        }
    }
    mark_all_dirty(dirty, state.placements.len());
    crate::app::input::refresh_panels(state, dirty);
    crate::app::annotation_selection::sync(state, dirty);
}
