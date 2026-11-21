//! 【贴图剪贴板】【关闭交接】关闭窗口并释放图像任务，保留数据来源直到接管

use super::runtime::Pin;
use crate::{
    backend::wayland::utils::shm_pool::SlotPool, features::pinned_image::model::PinnedImage,
};
use smithay_client_toolkit::reexports::calloop::EventLoop;
use wayland_client::Connection;

/// 【贴图剪贴板】【后台保留】按 pin、connection 和 event_loop 释放业务资源并驱动数据传输，返回关闭结果
pub(super) fn retain(
    mut pin: Pin,
    connection: Connection,
    mut event_loop: EventLoop<'static, Pin>,
) -> crate::AppResult {
    if let Some(core) = pin.clipboard.clone().filter(|core| core.borrow().active()) {
        // 1. 【贴图剪贴板】【窗口释放】关闭可见表面和后台任务，仅保留选择数据及协议分派状态
        pin.popup = None;
        pin.preview = None;
        pin.shell.surface().attach(None, 0, 0);
        pin.shell.surface().commit();
        pin.buffers.clear();
        pin.pool = SlotPool::new(4096, &pin.shm)?;
        pin.text.clear();
        pin.translation.clear();
        pin.generation = pin.generation.wrapping_add(1);
        pin.copy_after_ocr = false;
        pin.font_system = cosmic_text::FontSystem::new_with_fonts([]);
        pin.swash = cosmic_text::SwashCache::new();
        pin.icons.clear();
        if let Some(cancel) = pin.ocr_cancel.take() {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(mut task) = pin.translation_task.take() {
            task.shutdown();
        }
        for mut task in pin.translation_retired.drain(..) {
            task.shutdown();
        }
        let blank = tiny_skia::Pixmap::new(1, 1).ok_or("cannot release pin image")?;
        pin.image = PinnedImage::decode(&crate::utils::encode_png_with_alpha(blank), 1.0)?;
        connection.flush()?;
        crate::backend::wayland::clipboard::background::defer(move || {
            while core.borrow().active() {
                core.borrow_mut().progress();
                if !core.borrow().active() {
                    break;
                }
                let timeout = core
                    .borrow()
                    .timeout()
                    .map(|_| std::time::Duration::from_millis(5));
                event_loop.dispatch(timeout, &mut pin)?;
            }
            Ok(())
        });
    }
    Ok(())
}
