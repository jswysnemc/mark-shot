//! 【贴图】【分数缩放】高分屏使用精确比例的物理缓冲，不改变原图或窗口逻辑尺寸

use super::runtime::Pin;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1, wp_fractional_scale_v1,
};

impl Dispatch<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1, ()> for Pin {
    /// 【贴图】【分数缩放管理器】管理器没有业务事件，不额外创建缓冲
    fn event(
        _: &mut Self,
        _: &wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
        _: wp_fractional_scale_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wp_fractional_scale_v1::WpFractionalScaleV1, ()> for Pin {
    /// 【贴图】【比例通知】应用 fractional 的首选 scale；旧表面的通知不影响当前角色
    fn event(
        state: &mut Self,
        fractional: &wp_fractional_scale_v1::WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            if state.fractional.as_ref() == Some(fractional) {
                state.render_scale = (scale as f32 / 120.0).max(0.25);
                state.dirty = true;
            } else if let Some(preview) = &mut state.preview
                && preview.fractional.as_ref() == Some(fractional)
            {
                preview.scale = (scale as f32 / 120.0).max(0.25);
                preview.dirty = true;
            }
        }
    }
}
