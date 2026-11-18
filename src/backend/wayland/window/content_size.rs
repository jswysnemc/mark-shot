//! 【原生窗口】【内容尺寸】应用内容主动请求的逻辑尺寸，保持视区、窗口几何和界面布局同步

use super::{WindowContent, WindowEvent, runtime::NativeWindow, surface::SurfaceId};
use smithay_client_toolkit::shell::xdg::XdgSurface;

#[cfg(test)]
#[path = "content_size_tests.rs"]
mod tests;

impl NativeWindow {
    /// 【原生窗口】【主动调整】无参数，同步主窗口及存活模态子窗口的内容尺寸，无返回值
    pub(super) fn sync_content_size(&mut self) {
        self.sync_content_size_for(SurfaceId::Main);
        if let Some(id) = self.dialog.as_ref().map(|dialog| dialog.id) {
            self.sync_content_size_for(SurfaceId::Dialog(id));
        }
    }

    /// 【原生窗口】【所属尺寸】按 id 应用对应内容的尺寸请求并通知实际布局，未配置或过期身份不处理，无返回值
    fn sync_content_size_for(&mut self, id: SurfaceId) {
        let Some(requested) = self.content_for(id).and_then(WindowContent::preferred_size) else {
            return;
        };
        let Some(surface) = self.surface_mut(id).filter(|surface| surface.configured) else {
            return;
        };
        let size = (
            requested.0.max(surface.spec.minimum.0),
            requested.1.max(surface.spec.minimum.1),
        );
        if size == surface.size {
            return;
        }
        // 1. 【原生窗口】【协议几何】下一帧使用新逻辑尺寸，分数缩放继续由表面维护
        surface.size = size;
        if let Some(viewport) = &surface.viewport {
            viewport.set_destination(size.0 as i32, size.1 as i32);
        }
        if let Some(window) = surface.role.as_window() {
            window.set_window_geometry(0, 0, size.0, size.1);
        }
        surface.dirty = true;
        if id == SurfaceId::Main && self.main.role.as_window().is_none() {
            self.layer_geometry.size = size;
            self.main.role.place(self.layer_geometry);
        }
        // 2. 【原生窗口】【布局同步】内容先收到实际尺寸，再生成新帧，避免高级选项与底部按钮相互覆盖
        self.dispatch_to(id, WindowEvent::Resize { size }, None);
    }
}
