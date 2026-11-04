//! 【原生窗口】【模态协议】可用时声明 xdg-dialog，协议未提供时仍由客户端过滤父窗口输入

use super::runtime::NativeWindow;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::xdg::dialog::v1::client::{xdg_dialog_v1, xdg_wm_dialog_v1};

impl Dispatch<xdg_wm_dialog_v1::XdgWmDialogV1, ()> for NativeWindow {
    /// 【原生窗口】【对话框管理器】管理器无事件，参数来自当前连接，不触发业务操作，无返回值
    fn event(
        _: &mut Self,
        _: &xdg_wm_dialog_v1::XdgWmDialogV1,
        _: xdg_wm_dialog_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<xdg_dialog_v1::XdgDialogV1, ()> for NativeWindow {
    /// 【原生窗口】【模态提示】模态对象无事件，参数来自当前连接，不替代客户端输入隔离，无返回值
    fn event(
        _: &mut Self,
        _: &xdg_dialog_v1::XdgDialogV1,
        _: xdg_dialog_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
