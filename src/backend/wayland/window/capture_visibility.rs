//! 【截图窗口】【Wayland 显隐】临时解除表面映射，恢复时重新执行初始配置握手

use super::{runtime::NativeWindow, surface::Surface};

impl Surface {
    /// 【截图窗口】【表面显隐】按 hidden 隐藏或请求重新配置，不释放业务对象、原图及编辑历史
    fn capture_visibility(&mut self, hidden: bool) {
        self.configured = false;
        self.frame_pending = false;
        self.dirty = true;
        if hidden {
            self.role.wl_surface().attach(None, 0, 0);
        } else if let Some(window) = self.role.as_window() {
            window.set_title(self.spec.title.clone());
            window.set_app_id(self.spec.app_id.clone());
            window.set_min_size(Some(self.spec.minimum));
        }
        self.role.commit();
    }
}

impl NativeWindow {
    /// 【截图窗口】【窗口组显隐】按 hidden 同步主窗口和模态子窗口，取消旧菜单与输入抓取，无返回值
    pub(super) fn capture_visibility(&mut self, hidden: bool) {
        self.close_popups();
        self.pointer_grabs.cancel();
        self.keyboard_grabs.cancel();
        self.layer_gesture = None;
        if !hidden {
            self.main.role.place(self.layer_geometry);
            if let Some(layer) = self.main.role.as_layer() {
                use smithay_client_toolkit::shell::wlr_layer::{KeyboardInteractivity, Layer};
                layer.set_layer(Layer::Top);
                layer.set_exclusive_zone(-1);
                layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
            }
        }
        self.main.capture_visibility(hidden);
        if let Some(dialog) = &mut self.dialog {
            dialog.surface.capture_visibility(hidden);
        }
    }
}
