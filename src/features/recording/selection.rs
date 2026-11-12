//! 【录制选区】【截图交接】复用现有框选，只交付物理区域，不导出或缓存截图

use super::Target;

/// 【录制选区】【坐标映射】按 state 与 overlay 返回单个输出内的物理像素区域，取消返回空值
pub(crate) fn selected(
    state: &crate::editor::EditorState,
    overlay: &dyn crate::backend::ScreenOverlay,
) -> crate::AppResult<Option<Target>> {
    if state.cancel || state.finish.is_none() {
        return Ok(None);
    }
    let region = state
        .selection
        .zone
        .ok_or("没有录制区域 / No recording region")?;
    let (index, crop) = region_crop(region, &state.placements)?;
    let output = overlay
        .surface_output(index)
        .ok_or("录制显示器已移除 / Display removed")?;
    Ok(Some(Target {
        display: crate::backend::output_selection::name(output, index),
        crop: Some(crop),
        format: state.capture_start.recording,
    }))
}

/// 【录制选区】【区域换算】按全局逻辑 region 和 placements 返回所属表面序号及物理裁剪区域，跨屏返回错误
fn region_crop(
    region: tiny_skia::Rect,
    placements: &[crate::types::Placement],
) -> crate::AppResult<(usize, [u32; 4])> {
    for (index, placement) in placements.iter().enumerate() {
        let bounds = tiny_skia::Rect::from_xywh(
            placement.position.0 as f32,
            placement.position.1 as f32,
            placement.size.0 as f32,
            placement.size.1 as f32,
        )
        .ok_or("invalid display bounds")?;
        if region.left() >= bounds.left()
            && region.top() >= bounds.top()
            && region.right() <= bounds.right()
            && region.bottom() <= bounds.bottom()
        {
            let x = ((region.left() - bounds.left()) * placement.scale).round() as u32;
            let y = ((region.top() - bounds.top()) * placement.scale).round() as u32;
            let width = (region.width() * placement.scale).round() as u32 & !1;
            let height = (region.height() * placement.scale).round() as u32 & !1;
            if width < 2 || height < 2 {
                return Err("录制区域至少为 2×2 / Recording region is too small".into());
            }
            return Ok((index, [x, y, width, height]));
        }
    }
    Err("请在单个显示器内选择录制区域 / Select a region within one display".into())
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;

/// 【录制选区】【独立启动】按 target 启动轻量控制窗口，截图进程随后退出释放全部像素
pub(crate) fn spawn(target: &Target) -> crate::AppResult {
    let value = serde_json::to_string(target)?;
    crate::utils::spawn_self(&["--record", "--record-target", &value], &[])
}
