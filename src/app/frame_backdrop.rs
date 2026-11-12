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

/// 【截图绘制】【背景提交】按editor、overlay、monitor和damage交付帧，reclaim_transparent控制透明页检查，返回协议结果；其他接口保留原所有权
pub(super) fn stage(
    editor: &mut EditorState,
    overlay: &mut dyn ScreenOverlay,
    monitor: usize,
    damage: Option<DamageRect>,
    reclaim_transparent: bool,
) -> crate::AppResult {
    if editor.region_rendering.native_scene {
        let identity = editor.canvas_transform().is_identity();
        if identity && editor.region_rendering.baked_background {
            crate::renderer::opaque_backdrop::compose(
                &editor.captures[monitor].pixmap,
                &mut editor.canvas[monitor],
                damage,
            );
        }
        let placement = &editor.placements[monitor];
        let local = |rect| crate::tools::selection::global_selection_to_local(&rect, placement);
        let selection = editor.selection.zone.and_then(local);
        let hovered = editor
            .region_detection
            .hovered
            .filter(|_| editor.region_rendering.native_hover)
            .and_then(local);
        let opaque = !identity || editor.region_rendering.baked_background;
        overlay.stage_capture_scene(
            monitor,
            crate::types::capture_scene::Frame {
                foreground: &mut editor.canvas[monitor],
                background: &editor.captures[monitor].pixmap,
                damage,
                selection,
                hovered,
                opaque,
            },
        )?;
        if !opaque && reclaim_transparent {
            let size = &editor.canvas[monitor];
            let (x, y, width, height) = damage.unwrap_or((0, 0, size.width(), size.height()));
            if let Some(area) = tiny_skia::IntRect::from_xywh(x as i32, y as i32, width, height) {
                // 1. 【截图场景】【透明页归还】交付成功并获得定期检查许可后回收相交全零页，保留非零前景及尚未提交输出
                crate::utils::transparent_pixmap::reclaim_region(&mut editor.canvas[monitor], area);
            }
        }
        return Ok(());
    }
    if !editor.canvas_transform().is_identity() {
        return overlay.stage_opaque_frame(monitor, editor.canvas[monitor].data(), damage);
    }
    if editor.region_rendering.native_background {
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
