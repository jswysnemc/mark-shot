//! 【输入法】【原生绑定】按键盘 seat 创建可选 text-input-v3，主表面焦点与正文目标分别核验

mod events;
pub(crate) mod message;
mod state;
#[cfg(test)]
mod tests;

use wayland_client::{
    Dispatch, QueueHandle,
    protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
};
use wayland_protocols::wp::text_input::zv3::client::{
    zwp_text_input_manager_v3::ZwpTextInputManagerV3,
    zwp_text_input_v3::{
        ChangeCause as ProtocolCause, ContentHint, ContentPurpose, ZwpTextInputV3,
    },
};

use crate::types::text_input::{Focus, Snapshot, Update};
use state::{Request, State};

#[derive(Default)]
pub struct TextInput {
    pub manager: Option<ZwpTextInputManagerV3>,
    devices: Vec<Device>,
}

struct Device {
    proxy: ZwpTextInputV3,
    seat: WlSeat,
    surface: Option<WlSurface>,
    focus: Option<Focus>,
    state: State,
}

impl TextInput {
    /// 【输入法】【键盘绑定】用 seat 与 qh 创建唯一协议对象，无返回值；合成器不提供协议时保持普通键盘输入
    pub fn bind<D>(&mut self, seat: &WlSeat, qh: &QueueHandle<D>)
    where
        D: Dispatch<ZwpTextInputV3, ()> + 'static,
    {
        if self.devices.iter().any(|device| &device.seat == seat) {
            return;
        }
        let Some(manager) = self.manager.as_ref() else {
            return;
        };
        self.devices.push(Device {
            proxy: manager.get_text_input(seat, qh, ()),
            seat: seat.clone(),
            surface: None,
            focus: None,
            state: State::default(),
        });
    }

    /// 【输入法】【设备移除】销毁 seat 的协议对象并返回原生焦点离开事件，供键盘能力或设备移除边界调用
    pub fn remove(&mut self, seat: &WlSeat) -> Option<Focus> {
        let index = self
            .devices
            .iter()
            .position(|device| &device.seat == seat)?;
        let device = self.devices.remove(index);
        device.proxy.destroy();
        device.focus
    }

    /// 【输入法】【协议同步】将 snapshot 发给匹配的原生焦点，空值停用所有对象，无返回值；只更新实际变化的协议状态
    pub fn sync(&mut self, snapshot: Option<&Snapshot>) {
        for device in &mut self.devices {
            let snapshot = snapshot.filter(|snapshot| {
                device.focus.is_some_and(|focus| {
                    snapshot.focus_id == focus.id && snapshot.monitor_idx == focus.monitor_idx
                })
            });
            for request in device.state.sync(snapshot) {
                match request {
                    Request::Enable => device.proxy.enable(),
                    Request::Disable => device.proxy.disable(),
                    Request::Surrounding(value) => device.proxy.set_surrounding_text(
                        value.text.clone(),
                        value.cursor,
                        value.anchor,
                    ),
                    Request::Cause(cause) => device.proxy.set_text_change_cause(match cause {
                        crate::types::text_input::ChangeCause::InputMethod => {
                            ProtocolCause::InputMethod
                        }
                        crate::types::text_input::ChangeCause::Other => ProtocolCause::Other,
                    }),
                    Request::ContentType(content) => {
                        use crate::types::text_input::ContentType;
                        let (hint, purpose) = match content {
                            ContentType::Multiline => (
                                ContentHint::Completion
                                    | ContentHint::Spellcheck
                                    | ContentHint::Multiline,
                                ContentPurpose::Normal,
                            ),
                            ContentType::Text => (
                                ContentHint::Completion | ContentHint::Spellcheck,
                                ContentPurpose::Normal,
                            ),
                            ContentType::Digits => (ContentHint::None, ContentPurpose::Digits),
                            ContentType::Number => (ContentHint::None, ContentPurpose::Number),
                        };
                        device.proxy.set_content_type(hint, purpose);
                    }
                    Request::Cursor((x, y, width, height)) => {
                        device.proxy.set_cursor_rectangle(x, y, width, height)
                    }
                    Request::Commit => device.proxy.commit(),
                }
            }
        }
    }

    /// 【输入法】【交付复核】用当前对象提交计数重新计算 update 的同步许可，无返回值；前一排队事件可能已经触发新的客户端提交
    pub fn validate(&self, update: &mut Update) {
        update.sync_allowed = self.devices.iter().any(|device| {
            device
                .state
                .current
                .as_ref()
                .is_some_and(|current| current.target_id == update.target_id)
                && device.state.commit_count == update.serial
        });
    }
}

impl Drop for TextInput {
    /// 【输入法】【资源释放】销毁全部设备及管理器，无参数、无返回值；协议对象不跨应用生命周期保留
    fn drop(&mut self) {
        for device in &self.devices {
            device.proxy.destroy();
        }
        if let Some(manager) = self.manager.take() {
            manager.destroy();
        }
    }
}
