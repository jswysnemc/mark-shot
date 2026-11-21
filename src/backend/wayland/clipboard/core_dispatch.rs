//! 【Wayland剪贴板】【协议分派】覆盖层和贴图窗口共用核心剪贴板事件处理

macro_rules! clipboard_dispatch {
    ($state:ty) => { crate::backend::wayland::clipboard::core_dispatch::clipboard_dispatch!($state, |_state: &mut $state| {}); };
    ($state:ty, $cancelled:expr) => {
        impl wayland_client::Dispatch<wayland_client::protocol::wl_data_device_manager::WlDataDeviceManager, ()> for $state {
            /// 【Wayland剪贴板】【管理器事件】参数来自协议，管理器无事件，无返回值
            fn event(_: &mut Self, _: &wayland_client::protocol::wl_data_device_manager::WlDataDeviceManager, _: wayland_client::protocol::wl_data_device_manager::Event, _: &(), _: &wayland_client::Connection, _: &wayland_client::QueueHandle<Self>) {}
        }
        impl wayland_client::Dispatch<wayland_client::protocol::wl_data_source::WlDataSource, ()> for $state {
            /// 【Wayland剪贴板】【来源事件】按 source 和 event 发送数据或回收选择，其余参数来自协议，无返回值
            fn event(state: &mut Self, source: &wayland_client::protocol::wl_data_source::WlDataSource, event: wayland_client::protocol::wl_data_source::Event, _: &(), _: &wayland_client::Connection, _: &wayland_client::QueueHandle<Self>) {
                let Some(core) = &state.clipboard else { return; };
                match event {
                    wayland_client::protocol::wl_data_source::Event::Send { mime_type, fd } => core.borrow_mut().send(source, &mime_type, fd),
                    wayland_client::protocol::wl_data_source::Event::Cancelled => {
                        core.borrow_mut().cancelled(source);
                        ($cancelled)(state);
                    },
                    _ => {},
                }
            }
        }
        impl wayland_client::Dispatch<wayland_client::protocol::wl_data_device::WlDataDevice, ()> for $state {
            wayland_client::event_created_child!($state, wayland_client::protocol::wl_data_device::WlDataDevice, [0 => (wayland_client::protocol::wl_data_offer::WlDataOffer, ())]);
            /// 【Wayland剪贴板】【设备事件】按 event 回收不读取的选择或拖放来源，其余参数来自协议，无返回值
            fn event(_: &mut Self, _: &wayland_client::protocol::wl_data_device::WlDataDevice, event: wayland_client::protocol::wl_data_device::Event, _: &(), _: &wayland_client::Connection, _: &wayland_client::QueueHandle<Self>) {
                match event {
                    wayland_client::protocol::wl_data_device::Event::Selection { id: Some(offer) } => offer.destroy(),
                    wayland_client::protocol::wl_data_device::Event::Enter { serial, id: Some(offer), .. } => { offer.accept(serial, None); offer.destroy(); },
                    _ => {},
                }
            }
        }
        impl wayland_client::Dispatch<wayland_client::protocol::wl_data_offer::WlDataOffer, ()> for $state {
            /// 【Wayland剪贴板】【类型事件】参数来自协议，本模块只发送内容，不接收选择，无返回值
            fn event(_: &mut Self, _: &wayland_client::protocol::wl_data_offer::WlDataOffer, _: wayland_client::protocol::wl_data_offer::Event, _: &(), _: &wayland_client::Connection, _: &wayland_client::QueueHandle<Self>) {}
        }
    };
}
pub(crate) use clipboard_dispatch;

clipboard_dispatch!(
    crate::backend::wayland::overlay::state::OverlayState,
    |state: &mut crate::backend::wayland::overlay::state::OverlayState| {
        state.events.push_back(crate::types::OverlayEvent::Tick);
    }
);
