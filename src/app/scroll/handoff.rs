//! 【滚动截图】【结果交接】原始长图独立于缩略图，取消保存后回到同一捕获会话

use super::{PendingOutcome as Outcome, ScrollRun};
use crate::{
    backend::ScreenOverlay,
    features::save_as::{self, Snapshot},
};
use std::path::{Path, PathBuf};

/// 【滚动截图】【标注交接】转交run的原图块及原输出索引，返回待完成结果，捕获关闭后再转换，预览不参与交接
pub(super) fn annotate(run: &mut ScrollRun) -> crate::AppResult<Outcome> {
    let frames = run.session.frame_count;
    let image = run
        .session
        .stitcher
        .take_image()
        .ok_or("the scroll image is empty")?;
    log::info!(
        "【滚动截图】【标注交接】原图={}x{}，帧数={}",
        image.width(),
        image.height(),
        frames
    );
    Ok(Outcome::Annotate {
        image,
        monitor_idx: run.session.monitor_idx,
    })
}

/// 【滚动截图】【复制交接】转交run的原图块和设备比例，返回待复制结果，编码在捕获关闭后执行
pub(super) fn copy(run: &mut ScrollRun) -> crate::AppResult<Outcome> {
    let image = run
        .session
        .stitcher
        .take_image()
        .ok_or("the scroll image is empty")?;
    Ok(Outcome::Copy {
        image,
        scale: run.scale,
    })
}

/// 【滚动截图】【保存对话框】暂停 run 并释放 overlay 后使用 dir 规划路径，返回已保存目标，取消后恢复原捕获状态
pub(super) fn save(
    run: &mut ScrollRun,
    overlay: &mut dyn ScreenOverlay,
    dir: Option<&Path>,
) -> crate::AppResult<Option<PathBuf>> {
    let image = run.session.source().ok_or("the scroll image is empty")?;
    let png = crate::scroll::png::encode(image)?;
    let scale = run.scale;
    let snapshot = Snapshot::new(png, scale, String::new())?;
    let path = crate::utils::screenshot_path::planned(dir)?;
    let paused = run.session.paused;
    run.session.set_paused(true);
    overlay.suspend()?;
    let result = save_as::run(snapshot, &path);
    if let Ok(save_as::Outcome::Saved(path)) = &result {
        return Ok(Some(path.clone()));
    }
    // 1. 【滚动截图】【取消恢复】只恢复原生浮层，底层页面和已拼接原图保持当前状态
    overlay.resume()?;
    run.session.set_paused(paused);
    match result {
        Ok(save_as::Outcome::Cancelled) => log::info!("【滚动截图】【保存取消】已恢复捕获会话"),
        Ok(save_as::Outcome::Failed(error)) => {
            log::error!("【滚动截图】【保存失败】{error}");
            run.session.set_paused(true);
            run.session.status = "Capture error".into();
        }
        Err(error) => {
            log::error!("【滚动截图】【保存窗口】{error}");
            run.session.set_paused(true);
            run.session.status = "Capture error".into();
        }
        Ok(save_as::Outcome::Saved(_)) => unreachable!(),
    }
    Ok(None)
}
