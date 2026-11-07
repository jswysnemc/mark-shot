//! 【截图绘制】【背景交接】普通编辑合成单一表面，冻结像素保留为共享只读来源

use crate::{
    backend::ScreenOverlay,
    editor::EditorState,
    types::{Capture, DamageRect, Placement},
};

#[cfg(test)]
#[path = "frame_backdrop_tests.rs"]
mod tests;

/// 【截图绘制】【原生像素检查】按captures与placements检查冻结背景和输出画布是否具有相同像素网格，返回是否无需新增缩放
pub(super) fn compatible(captures: &[Capture], placements: &[Placement]) -> bool {
    captures.len() == placements.len()
        && captures.iter().zip(placements).all(|(capture, placement)| {
            (capture.pixmap.width(), capture.pixmap.height()) == placement.px_size()
        })
}

/// 【截图绘制】【背景模式】按editor、overlay和preferred启用单表面，尺寸不一致时保留原生采样，返回值为空
pub(super) fn configure(
    editor: &mut EditorState,
    overlay: &mut dyn ScreenOverlay,
    preferred: bool,
) {
    let enabled = preferred && compatible(&editor.captures, &editor.placements);
    if !enabled {
        overlay.set_composited_background(false);
    }
    editor.region_rendering.opaque_background = enabled;
    editor.region_rendering.native_background =
        enabled && overlay.supports_native_frame_background();
}

/// 【截图绘制】【帧背景】按editor、monitor和damage在提交前合成冻结背景，仅处理已恢复范围，无返回值
pub(super) fn compose(editor: &mut EditorState, monitor: usize, damage: Option<DamageRect>) {
    if !editor.region_rendering.opaque_background || editor.region_rendering.native_background {
        return;
    }
    let Some(capture) = editor.captures.get(monitor) else {
        return;
    };
    crate::renderer::opaque_backdrop::compose(&capture.pixmap, &mut editor.canvas[monitor], damage);
}

/// 【截图绘制】【背景提交】按editor、overlay、monitor和damage交付帧，返回协议结果；明确支持消费时归还已交付前景，其他接口保留CPU内容
pub(super) fn stage(
    editor: &mut EditorState,
    overlay: &mut dyn ScreenOverlay,
    monitor: usize,
    damage: Option<DamageRect>,
) -> crate::AppResult {
    if editor.region_rendering.native_background {
        // 1. 【截图绘制】【复制前归还】损坏恢复可能写入大面积零值，先归还这些页，再让原生备用缓冲继承完整当前帧
        crate::utils::transparent_pixmap::reclaim_blank(&mut editor.canvas[monitor]);
        if overlay.supports_consumed_frame_background() {
            return overlay.stage_consumed_frame_with_background(
                monitor,
                &mut editor.canvas[monitor],
                &editor.captures[monitor].pixmap,
                damage,
            );
        }
        overlay.stage_frame_with_background(
            monitor,
            editor.canvas[monitor].data(),
            &editor.captures[monitor].pixmap,
            damage,
        )
    } else {
        compose(editor, monitor, damage);
        if !editor.region_rendering.opaque_background {
            crate::utils::transparent_pixmap::reclaim_blank(&mut editor.canvas[monitor]);
        }
        overlay.stage_frame(monitor, editor.canvas[monitor].data(), damage)
    }
}
