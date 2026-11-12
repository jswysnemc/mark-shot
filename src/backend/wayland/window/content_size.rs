//! 【原生窗口】【内容尺寸】应用内容主动请求的逻辑尺寸，保持视区、窗口几何和界面布局同步

use super::{WindowEvent, runtime::NativeWindow, surface::SurfaceId};
use smithay_client_toolkit::shell::xdg::XdgSurface;

impl NativeWindow {
    /// 【原生窗口】【主动调整】无参数，读取主内容的尺寸请求并同步协议几何，通知内容实际尺寸；没有请求或尚未配置时不处理
    pub(super) fn sync_content_size(&mut self) {
        let Some(size) = self
            .content
            .preferred_size()
            .filter(|_| self.main.configured)
        else {
            return;
        };
        let size = (
            size.0.max(self.main.spec.minimum.0),
            size.1.max(self.main.spec.minimum.1),
        );
        if size == self.main.size {
            return;
        }
        // 1. 【原生窗口】【协议几何】下一帧使用新逻辑尺寸，分数缩放继续由表面维护
        self.main.size = size;
        if let Some(viewport) = &self.main.viewport {
            viewport.set_destination(size.0 as i32, size.1 as i32);
        }
        if let Some(window) = self.main.role.as_window() {
            window.set_window_geometry(0, 0, size.0, size.1);
        } else {
            self.layer_geometry.size = size;
            self.main.role.place(self.layer_geometry);
        }
        self.main.dirty = true;
        // 2. 【原生窗口】【布局同步】内容先收到实际尺寸，再生成新帧，避免高级选项与底部按钮相互覆盖
        self.dispatch_to(SurfaceId::Main, WindowEvent::Resize { size }, None);
    }
}
