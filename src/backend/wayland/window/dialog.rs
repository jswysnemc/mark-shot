//! 【原生窗口】【父子对话框】同连接建立模态关系，过滤父窗口输入并只回传一次明确结果

use super::{
    WindowContent, WindowSpec,
    runtime::NativeWindow,
    surface::{Surface, SurfaceId},
};
use smithay_client_toolkit::shell::WaylandSurface;
use wayland_client::{QueueHandle, protocol::wl_surface};
use wayland_protocols::xdg::dialog::v1::client::xdg_dialog_v1::XdgDialogV1;

pub struct WindowDialog {
    pub id: u64,
    pub spec: WindowSpec,
    pub content: Box<dyn WindowContent>,
}

pub(super) struct Dialog {
    pub id: u64,
    pub surface: Surface,
    pub content: Box<dyn WindowContent>,
    pub modal: Option<XdgDialogV1>,
}

impl Drop for Dialog {
    /// 【原生窗口】【模态回收】在子表面析构前销毁模态提示，避免父窗口继续保持模态状态，无参数、无返回值
    fn drop(&mut self) {
        if let Some(modal) = self.modal.take() {
            modal.destroy();
        }
    }
}

impl NativeWindow {
    /// 【原生窗口】【表面身份】将 surface 映射到当前窗口或存活子窗口，过期及其他表面返回空值
    pub fn surface_id(&self, surface: &wl_surface::WlSurface) -> Option<SurfaceId> {
        if self.main.matches(surface) {
            Some(SurfaceId::Main)
        } else {
            self.dialog
                .as_ref()
                .filter(|dialog| dialog.surface.matches(surface))
                .map(|dialog| SurfaceId::Dialog(dialog.id))
        }
    }

    /// 【原生窗口】【表面查询】按 id 查询可变表面，已关闭的子窗口身份不匹配新窗口，返回空值
    pub fn surface_mut(&mut self, id: SurfaceId) -> Option<&mut Surface> {
        match id {
            SurfaceId::Main => Some(&mut self.main),
            SurfaceId::Dialog(id) => self
                .dialog
                .as_mut()
                .filter(|dialog| dialog.id == id)
                .map(|dialog| &mut dialog.surface),
        }
    }

    /// 【原生窗口】【内容查询】按 id 返回当前内容，关闭的子窗口不回退到父窗口
    pub fn content_for(&self, id: SurfaceId) -> Option<&dyn WindowContent> {
        match id {
            SurfaceId::Main => Some(self.content.as_ref()),
            SurfaceId::Dialog(id) => self
                .dialog
                .as_ref()
                .filter(|dialog| dialog.id == id)
                .map(|dialog| dialog.content.as_ref()),
        }
    }

    /// 【原生窗口】【候选清理】取消 id 表面的候选并标记必要重绘，无返回值，过期身份不影响新内容
    pub fn cancel_for(&mut self, id: SurfaceId) {
        let changed = match id {
            SurfaceId::Main => self.content.cancel_text_input(),
            SurfaceId::Dialog(id) => self
                .dialog
                .as_mut()
                .filter(|dialog| dialog.id == id)
                .is_some_and(|dialog| dialog.content.cancel_text_input()),
        };
        if changed && let Some(surface) = self.surface_mut(id) {
            surface.dirty = true;
        }
    }

    /// 【原生窗口】【对话框同步】取出父内容的待显示请求，用 qh 创建同连接子窗口，不修改业务结果，无返回值
    pub fn sync_dialog(&mut self, qh: &QueueHandle<Self>) {
        if self.dialog.is_some() || self.exit {
            return;
        }
        let Some(mut request) = self.content.take_dialog() else {
            return;
        };
        request.content.connected(self.wake.clone());
        // 1. 【原生窗口】【输入隔离】先取消原输入目标，新的子窗口不会继承父窗口候选或按钮按下事务
        self.cancel_for(SurfaceId::Main);
        if let Some(focus) = self.input_target.focus {
            self.leave_text_input(focus.id);
        }
        self.text_input.sync(None);
        let surface = Surface::new(
            request.spec,
            SurfaceId::Dialog(request.id),
            &self.compositor,
            &self.shell,
            self.viewporter.as_ref(),
            self.fractional_manager.as_ref(),
            qh,
        );
        surface.window.set_parent(Some(&self.main.window));
        let modal = self.dialog_manager.as_ref().map(|manager| {
            let dialog = manager.get_xdg_dialog(surface.window.xdg_toplevel(), qh, ());
            dialog.set_modal();
            dialog
        });
        // 2. 【原生窗口】【首次呈现】父子关系和模态提示在首次提交前设置，避免短暂显示为无关普通窗口
        surface.window.commit();
        log::info!("【原生窗口】【对话框打开】id={}", request.id);
        self.dialog = Some(Dialog {
            id: request.id,
            surface,
            content: request.content,
            modal,
        });
    }

    /// 【原生窗口】【对话框完成】只处理存活 id，accepted 表示用户明确确认；先回收子表面，再通知父内容，无返回值
    pub fn finish_dialog(&mut self, id: u64, accepted: bool) {
        if !self.dialog.as_ref().is_some_and(|dialog| dialog.id == id) {
            return;
        }
        if self.text_surface == Some(SurfaceId::Dialog(id)) {
            if let Some(focus) = self.input_target.focus {
                self.leave_text_input(focus.id);
            }
            self.text_surface = None;
        }
        self.dialog.take();
        log::info!("【原生窗口】【对话框完成】id={id}, accepted={accepted}");
        if self.keyboard_surface == Some(SurfaceId::Dialog(id)) {
            self.keyboard_surface = None;
        }
        let action = self.content.dialog_closed(id, accepted);
        self.apply(SurfaceId::Main, action, None);
    }
}

#[cfg(test)]
mod tests {
    use crate::backend::wayland::window::{KeyInput, WindowEvent};

    /// 【原生窗口测试】【模态事件】父窗口输入全部隔离，但允许后台、布局与焦点生命周期，无参数、无返回值
    #[test]
    fn modality_filters_interactions_not_lifecycle_events() {
        let key = KeyInput {
            key: None,
            text: String::new(),
            mods: Default::default(),
        };
        for event in [
            WindowEvent::Key(key.clone()),
            WindowEvent::KeyRelease(key),
            WindowEvent::PointerButton {
                position: (1.0, 2.0),
                button: super::super::RIGHT_BUTTON,
                pressed: true,
            },
            WindowEvent::Scroll {
                position: (1.0, 2.0),
                delta: (0.0, 10.0),
            },
        ] {
            assert!(event.is_interaction());
        }
        for event in [
            WindowEvent::Wake,
            WindowEvent::Resize { size: (800, 520) },
            WindowEvent::Focused(false),
        ] {
            assert!(!event.is_interaction());
        }
    }
}
