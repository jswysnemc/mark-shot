//! 【滚动截图】【真实捕获】按选区所属表面绑定输出，将逻辑区域映射到原始像素

use super::ScrollRun;
use crate::{backend::ScreenOverlay, types::MonitorFrame};
use tiny_skia::Rect;

/// 【滚动截图】【帧接收】将 result 送入 run 的拼接器，overlay 验证输出仍然存活，返回界面是否变化
pub(super) fn apply(
    run: &mut ScrollRun,
    overlay: &mut dyn ScreenOverlay,
    result: crate::types::CaptureResult,
) -> crate::AppResult<bool> {
    let _output = overlay
        .surface_output(run.session.monitor_idx)
        .ok_or("the scroll capture output is gone")?;
    let data = result
        .frames
        .into_iter()
        .next()
        .ok_or("the scroll capture backend returned an empty frame")?;
    if !overlay.allow_scroll_frame(&data, run.session.region)? {
        run.needs_clean_frame = true;
        return Ok(true);
    }
    run.needs_clean_frame = false;
    let scale = frame_scale(&data)?;
    let before = (
        run.session.stitcher.revision(),
        run.session.status.clone(),
        run.session.paused,
        run.session.capture_range,
    );
    run.scale = scale;
    run.session
        .push_capture(data, pixel_rect(run.session.region, scale))?;
    // 1. 【滚动截图】【暂停状态】收尾帧可以更新原图，但不能把已暂停界面改回正在捕获
    if run.session.paused && run.session.status != "Capture error" {
        run.session.status = "Paused".into();
    }
    Ok(before
        != (
            run.session.stitcher.revision(),
            run.session.status.clone(),
            run.session.paused,
            run.session.capture_range,
        ))
}

/// 【滚动截图】【物理裁切】以 region 和 scale 映射两端边界，返回原图像素矩形，避免分数比例宽度漂移
pub(super) fn pixel_rect(region: Rect, scale: f32) -> (u32, u32, u32, u32) {
    let x = (region.left() * scale).round().max(0.0) as u32;
    let y = (region.top() * scale).round().max(0.0) as u32;
    let right = (region.right() * scale).round().max((x + 1) as f32) as u32;
    let bottom = (region.bottom() * scale).round().max((y + 1) as f32) as u32;
    (x, y, right - x, bottom - y)
}

/// 【滚动截图】【比例读取】以 data 的实际帧和逻辑输出宽度返回捕获比例，无逻辑尺寸时报告错误
fn frame_scale(data: &MonitorFrame) -> crate::AppResult<f32> {
    let (width, _) = data
        .info
        .size
        .filter(|(w, h)| *w > 0 && *h > 0)
        .ok_or("scroll frame has no logical output dimensions")?;
    Ok(data.pw_width as f32 / width as f32)
}
