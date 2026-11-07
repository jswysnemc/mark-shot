//! 【原生窗口】【弹层树】逐级建立原生父子表面，逆序销毁全部相关抓取后交付业务动作

use super::{
    WindowPopupAction, popup_resources::WindowPopup, runtime::NativeWindow, surface::SurfaceId,
};
use crate::{
    AppResult,
    types::{popup::PopupEvent, popup_tree},
};
use smithay_client_toolkit::shell::xdg::{
    XdgSurface,
    popup::{Popup, PopupConfigure, PopupHandler},
};
use wayland_client::{
    Connection, QueueHandle,
    protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
};

impl NativeWindow {
    /// 【原生窗口】【弹层同步】按 qh 与当前 grab 同步完整父子链，子菜单仅继承存活父抓取，无返回值
    pub fn sync_popup(&mut self, qh: &QueueHandle<Self>, grab: Option<(&WlSeat, u32)>) {
        let nodes = self.content.popup_nodes();
        if !popup_tree::valid(&nodes, self.content.popup_id()) {
            log::error!("【原生窗口】【弹层同步】拒绝无效弹层父子链");
            self.close_popups();
            return;
        }
        // 1. 【原生窗口】【逆序回收】只保留同一父链和比例的前缀，退出的子菜单不能留下抓取
        let common = self
            .popup
            .iter()
            .zip(&nodes)
            .take_while(|(live, node)| {
                live.id == node.id && live.parent == node.parent && live.scale == self.main.scale
            })
            .count();
        if common < self.popup.len() {
            self.finish_popup(self.popup[common].id, None);
        }
        if self.dialog.is_some() || self.exit {
            self.close_popups();
            if let Some(id) = self.content.popup_id() {
                self.content.popup_closed(id, None);
            }
            return;
        }
        for (index, node) in nodes.into_iter().enumerate() {
            if index > self.popup.len() {
                break;
            }
            let Some(frame) = self.content.popup_frame_for(node.id, self.main.scale) else {
                continue;
            };
            let result = if index < self.popup.len() {
                self.popup[index].update(frame)
            } else {
                // 2. 【原生窗口】【抓取继承】根菜单必须拥有新的真实输入；子菜单复用的仍是当前父链内活动抓取
                let origin = if node.parent.is_some() {
                    self.popup.last().map(|parent| parent.grab.clone())
                } else {
                    grab.map(|(seat, serial)| (seat.clone(), serial))
                };
                let Some(origin) = origin else {
                    log::error!("【原生窗口】【弹层同步】根菜单没有有效输入抓取");
                    let action = self.content.popup_closed(node.id, None);
                    self.apply(SurfaceId::Main, action, None);
                    break;
                };
                let parent = if node.parent.is_some() {
                    self.popup.last().map(|parent| parent.popup.xdg_surface())
                } else {
                    self.main
                        .role
                        .as_window()
                        .map(|window| window.xdg_surface())
                };
                let layer = if node.parent.is_none() {
                    self.main.role.as_layer()
                } else {
                    None
                };
                WindowPopup::create(
                    frame,
                    node,
                    parent,
                    layer,
                    origin,
                    &self.compositor,
                    &self.shell,
                    self.viewporter.as_ref(),
                    qh,
                )
                .map(|popup| self.popup.push(popup))
            };
            if let Err(error) = result {
                log::error!("【原生窗口】【弹层同步】id={}, {error}", node.id);
                if index < self.popup.len() {
                    self.finish_popup(node.id, None);
                } else {
                    let action = self.content.popup_closed(node.id, None);
                    self.apply(SurfaceId::Main, action, None);
                }
                break;
            }
        }
        if !self.popup.is_empty() {
            self.pointer_grabs.cancel();
            self.keyboard_grabs.cancel();
        }
        self.reconcile_text_input();
    }

    /// 【原生窗口】【弹层完成】关闭 id 及其子菜单，action 为业务动作时先关闭整棵树再交付，旧编号无效，无返回值
    pub fn finish_popup(&mut self, id: u64, action: Option<usize>) {
        let Some(index) = self.popup.iter().position(|popup| popup.id == id) else {
            return;
        };
        let removed = self
            .popup
            .split_off(if action.is_some() { 0 } else { index });
        let ids = removed.iter().map(|popup| popup.id).collect::<Vec<_>>();
        // 1. 【原生窗口】【全部销毁】最深表面先销毁，任何动作都不能在祖先抓取仍存活时执行
        for popup in removed.into_iter().rev() {
            drop(popup);
        }
        // 2. 【原生窗口】【有序交付】取消子菜单不影响父菜单，业务动作只交付一次并保持原始会话编号
        for closed in ids.into_iter().rev() {
            let selected = (closed == id).then_some(action).flatten();
            log::info!("【原生窗口】【弹层关闭】id={closed}, action={selected:?}");
            let result = self.content.popup_closed(closed, selected);
            self.apply(SurfaceId::Main, result, None);
        }
        self.reconcile_text_input();
    }

    /// 【原生窗口】【整树关闭】取消当前根菜单和所有后代，无参数、无返回值
    pub fn close_popups(&mut self) {
        if let Some(id) = self.popup.first().map(|popup| popup.id) {
            self.finish_popup(id, None);
        }
    }

    /// 【原生窗口】【弹层事件】向存活 id 分派 event，关闭子层或整树由内容明确请求，无返回值
    pub fn popup_dispatch(&mut self, id: u64, event: PopupEvent) {
        if !self.popup.iter().any(|popup| popup.id == id) {
            return;
        }
        match self.content.popup_event(id, event) {
            WindowPopupAction::None | WindowPopupAction::Redraw => {}
            WindowPopupAction::Close => self.finish_popup(id, None),
            WindowPopupAction::CloseAll => self.close_popups(),
            WindowPopupAction::Activate(index) => self.finish_popup(id, Some(index)),
        }
    }

    /// 【原生窗口】【弹层绘制】用 qh 按父子顺序绘制全部存活表面，返回缓冲错误，不引入周期工作
    pub fn paint_popup(&mut self, qh: &QueueHandle<Self>) -> AppResult {
        for popup in &mut self.popup {
            popup.paint(&mut self.pool, qh)?;
        }
        Ok(())
    }

    /// 【原生窗口】【弹层帧许可】只解除 surface 对应节点的提交限制，返回是否消费，旧表面不影响新菜单
    pub fn popup_frame_done(&mut self, surface: &WlSurface) -> bool {
        let Some(popup) = self
            .popup
            .iter_mut()
            .find(|popup| popup.surface() == surface)
        else {
            return false;
        };
        popup.frame_done();
        true
    }
}

impl PopupHandler for NativeWindow {
    /// 【原生窗口】【弹层配置】接受 popup 的 configure 并交付该节点实际位置，无返回值，不把子菜单误当作根菜单
    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        popup: &Popup,
        configure: PopupConfigure,
    ) {
        let Some(current) = self
            .popup
            .iter_mut()
            .find(|current| &current.popup == popup)
        else {
            return;
        };
        current.configure();
        let id = current.id;
        self.popup_dispatch(
            id,
            PopupEvent::Configured {
                position: configure.position,
            },
        );
    }
    /// 【原生窗口】【合成器取消】关闭 popup 对应节点及后代，旧 done 不能影响新会话，无返回值
    fn done(&mut self, _: &Connection, _: &QueueHandle<Self>, popup: &Popup) {
        if let Some(id) = self
            .popup
            .iter()
            .find(|current| &current.popup == popup)
            .map(|current| current.id)
        {
            self.finish_popup(id, None);
        }
    }
}
