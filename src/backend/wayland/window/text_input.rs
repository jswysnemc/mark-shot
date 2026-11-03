//! 【原生窗口】【输入法衔接】原生焦点与内容身份共同确定输入目标，拒绝过期窗口或控件消息

use super::{WindowEvent, runtime::NativeWindow};
use crate::{
    backend::wayland::text_input::message::Message,
    features::text::input_method::focus::identity,
    types::text_input::{ChangeCause, Edit, Focus},
};
use smithay_client_toolkit::shell::WaylandSurface;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::text_input::zv3::client::{
    zwp_text_input_manager_v3, zwp_text_input_v3,
};

impl NativeWindow {
    /// 【原生窗口】【输入资格】按当前内容更新目标，取消旧候选并停用无主协议，无参数、无返回值
    pub(super) fn reconcile_text_input(&mut self) {
        if self
            .input_target
            .reconcile(self.content.text_input_target())
            && self.content.cancel_text_input()
        {
            self.dirty = true;
        }
        if self.input_target.id.is_none() {
            self.text_input.sync(None);
        }
    }

    /// 【原生窗口】【上下文交付】同步实际变化的输入快照，过期序号等待匹配确认，无参数、无返回值
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
        if let Some(snapshot) = self.content.text_input_snapshot(
            focus,
            target_id,
            target.snapshot.as_ref(),
            target.cause,
        ) {
            self.text_input.sync(Some(&snapshot));
            target.snapshot = Some(snapshot);
            target.cause = ChangeCause::Other;
        }
    }

    /// 【原生窗口】【原生离开】仅在 focus_id 匹配当前焦点时取消候选，返回是否匹配；旧消息不能清理新输入
    pub(super) fn leave_text_input(&mut self, focus_id: u64) -> bool {
        if !self.input_target.leave(focus_id) {
            return false;
        }
        self.dirty |= self.content.cancel_text_input();
        true
    }
}

impl Dispatch<zwp_text_input_manager_v3::ZwpTextInputManagerV3, ()> for NativeWindow {
    /// 【原生窗口】【输入管理器】管理器无事件，协议参数不产生业务动作，无返回值
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
    /// 【原生窗口】【原子输入】处理 proxy 的 event，只接收本窗口焦点与当前目标，其余参数来自协议，无返回值
    fn event(
        state: &mut Self,
        proxy: &zwp_text_input_v3::ZwpTextInputV3,
        event: zwp_text_input_v3::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let surface = state.window.wl_surface();
        let message = state.text_input.message(proxy, event, |candidate| {
            (candidate == surface).then(|| Focus {
                id: identity(),
                monitor_idx: 0,
            })
        });
        match message {
            Some(Message::Focus(focus)) => {
                state.dirty |= state.content.cancel_text_input();
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
                {
                    state.dispatch(WindowEvent::TextInput(edit), None);
                    state.input_target.cause = ChangeCause::InputMethod;
                }
            }
            None => {}
        }
    }
}
