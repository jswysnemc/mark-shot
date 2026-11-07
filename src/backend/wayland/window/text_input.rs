//! 【原生窗口】【输入法衔接】按父子表面身份更新目标，拒绝过期窗口或控件批次

use super::{
    WindowContent, WindowEvent, dialog::Dialog, runtime::NativeWindow, surface::SurfaceId,
};
use crate::{
    backend::wayland::text_input::message::Message,
    features::text::input_method::focus::identity,
    types::text_input::{ChangeCause, Edit, Focus},
};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::text_input::zv3::client::{
    zwp_text_input_manager_v3, zwp_text_input_v3,
};

/// 【原生窗口】【可编辑内容】从 main、dialog 和 id 返回当前可编辑表面；模态时不返回父内容，过期身份返回空值
fn input_content<'a>(
    main: &'a mut Box<dyn WindowContent>,
    dialog: &'a mut Option<Dialog>,
    id: Option<SurfaceId>,
) -> Option<&'a mut (dyn WindowContent + 'static)> {
    match id {
        Some(SurfaceId::Main) if dialog.is_none() => Some(main.as_mut()),
        Some(SurfaceId::Dialog(id)) => match dialog.as_mut() {
            Some(dialog) if dialog.id == id => Some(dialog.content.as_mut()),
            _ => None,
        },
        _ => None,
    }
}

impl NativeWindow {
    /// 【原生窗口】【输入资格】按原生焦点所属表面更新目标，停用旧交互而由内容决定候选外观，无参数、无返回值
    pub(super) fn reconcile_text_input(&mut self) {
        let target = if !self.popup.is_empty() {
            None
        } else {
            input_content(&mut self.content, &mut self.dialog, self.text_surface)
                .and_then(|content| content.text_input_target())
        };
        if self.input_target.reconcile(target)
            && let Some(id) = self.text_surface
        {
            self.suspend_for(id);
        }
        if self.input_target.id.is_none() {
            self.text_input.sync(None);
        }
    }

    /// 【原生窗口】【内容停用】暂停 id 对应内容的输入交互，必要时标记重绘；过期身份不影响新表面，无返回值
    fn suspend_for(&mut self, id: SurfaceId) {
        let changed = input_content(&mut self.content, &mut self.dialog, Some(id))
            .is_some_and(|content| content.suspend_text_input());
        if changed && let Some(surface) = self.surface_mut(id) {
            surface.dirty = true;
        }
    }

    /// 【原生窗口】【上下文交付】同步实际表面变化的有界输入快照，过期序号等待匹配确认，无参数、无返回值
    pub(super) fn sync_text_input(&mut self) {
        self.reconcile_text_input();
        let target = &mut self.input_target;
        if !target.sync_allowed {
            return;
        }
        let (Some(target_id), Some(focus)) = (target.id, target.focus) else {
            self.text_input.sync(None);
            return;
        };
        let Some(content) = input_content(&mut self.content, &mut self.dialog, self.text_surface)
        else {
            self.text_input.sync(None);
            return;
        };
        if let Some(snapshot) =
            content.text_input_snapshot(focus, target_id, target.snapshot.as_ref(), target.cause)
        {
            self.text_input.sync(Some(&snapshot));
            target.snapshot = Some(snapshot);
            target.cause = ChangeCause::Other;
        }
    }

    /// 【原生窗口】【原生离开】仅在 focus_id 匹配当前焦点时取消所属表面候选，返回是否匹配，旧消息不清理新输入
    pub(super) fn leave_text_input(&mut self, focus_id: u64) -> bool {
        if !self.input_target.leave(focus_id) {
            return false;
        }
        if let Some(id) = self.text_surface.take() {
            self.cancel_for(id);
        }
        true
    }
}

impl Dispatch<zwp_text_input_manager_v3::ZwpTextInputManagerV3, ()> for NativeWindow {
    /// 【原生窗口】【输入管理器】管理器无事件，参数来自当前连接，不产生业务动作，无返回值
    fn event(
        _: &mut Self,
        _: &zwp_text_input_manager_v3::ZwpTextInputManagerV3,
        _: zwp_text_input_manager_v3::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwp_text_input_v3::ZwpTextInputV3, ()> for NativeWindow {
    /// 【原生窗口】【原子输入】处理 proxy 的 event，仅接受当前表面焦点和目标，其余参数来自协议，无返回值
    fn event(
        state: &mut Self,
        proxy: &zwp_text_input_v3::ZwpTextInputV3,
        event: zwp_text_input_v3::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let main = state.main.role.wl_surface();
        let dialog = state
            .dialog
            .as_ref()
            .map(|dialog| (dialog.id, dialog.surface.role.wl_surface()));
        // 1. 【原生窗口】【输入归属】独立记录表面身份，不把子窗口身份当作输出索引
        let mut entered = None;
        let popup_open = !state.popup.is_empty();
        let message = state.text_input.message(proxy, event, |candidate| {
            entered = if dialog.is_none() && !popup_open && candidate == main {
                Some(SurfaceId::Main)
            } else {
                dialog
                    .filter(|(_, surface)| candidate == *surface)
                    .map(|(id, _)| SurfaceId::Dialog(id))
            };
            entered.map(|_| Focus {
                id: identity(),
                monitor_idx: 0,
            })
        });
        match message {
            Some(Message::Focus(focus)) => {
                if let Some(id) = state.text_surface {
                    state.cancel_for(id);
                }
                state.text_surface = entered;
                state.input_target.enter(focus);
                state.reconcile_text_input();
            }
            Some(Message::Leave(focus_id)) => {
                state.leave_text_input(focus_id);
            }
            Some(Message::Edit(mut update)) => {
                state.reconcile_text_input();
                state.text_input.validate(&mut update);
                if state.input_target.accept(&update)
                    && let Some(edit) = Edit::from_batch(update.batch)
                    && let Some(id) = state.text_surface
                {
                    state.dispatch_to(id, WindowEvent::TextInput(edit), None);
                    state.input_target.cause = ChangeCause::InputMethod;
                }
            }
            None => {}
        }
    }
}
