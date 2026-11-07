//! 【原生弹层】【协议资源】一个弹层的定位器、活动父抓取、视区和有界缓冲

use super::{runtime::NativeWindow, surface::SurfaceId};
use crate::{
    AppResult,
    backend::wayland::utils::buffers,
    types::{
        popup::{PopupFrame, validate},
        popup_tree::{Placement, PopupNode},
    },
};
use smithay_client_toolkit::{
    compositor::FrameCallbackData,
    shell::wlr_layer::LayerSurface,
    shell::xdg::{XdgPositioner, popup::Popup},
    shm::slot::{Buffer, SlotPool},
};
use wayland_client::{
    QueueHandle,
    protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
};
use wayland_protocols::{
    wp::viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
    xdg::shell::client::{
        xdg_positioner::{Anchor, ConstraintAdjustment, Gravity},
        xdg_surface::XdgSurface as ParentSurface,
    },
};

pub(super) struct WindowPopup {
    pub id: u64,
    pub parent: Option<u64>,
    pub popup: Popup,
    pub keyboard_focused: bool,
    pub scale: f32,
    pub grab: (WlSeat, u32),
    configured: bool,
    size: (u32, u32),
    pixel_size: (u32, u32),
    pixels: Vec<u8>,
    buffers: Vec<Buffer>,
    viewport: Option<WpViewport>,
    dirty: bool,
    frame_pending: bool,
}

impl Drop for WindowPopup {
    /// 【原生弹层】【资源释放】先销毁视区，再由工具包销毁当前角色和表面，无参数、无返回值
    fn drop(&mut self) {
        if let Some(viewport) = self.viewport.take() {
            viewport.destroy();
        }
    }
}

/// 【原生弹层】【锚定矩形】按 placement 和 anchor 返回协议整数矩形，控件与子菜单继续使用各自对齐规则
fn anchor_rect(placement: Placement, anchor: (f32, f32)) -> (i32, i32, i32, i32) {
    match placement {
        Placement::Point => (anchor.0.floor() as i32, anchor.1.floor() as i32, 1, 1),
        Placement::BelowControl(rect) => (
            rect.x().round() as i32,
            rect.y().round() as i32 - 4,
            rect.width().round() as i32,
            rect.height().round() as i32 + 8,
        ),
        Placement::Submenu {
            parent_width,
            row_top,
            item_inset,
        } => (
            2,
            (row_top - item_inset).round() as i32,
            parent_width.round() as i32 - 4,
            1,
        ),
    }
}

/// 【原生弹层】【定位规则】按 node 和 frame 设置 positioner，子菜单只允许水平翻转及垂直贴边，无返回值
fn position(positioner: &XdgPositioner, node: PopupNode, frame: &PopupFrame<'_>) {
    let rect = anchor_rect(node.placement, frame.anchor);
    positioner.set_anchor_rect(rect.0, rect.1, rect.2, rect.3);
    let anchor = match node.placement {
        Placement::Point => Anchor::TopLeft,
        Placement::BelowControl(_) => Anchor::BottomLeft,
        Placement::Submenu { .. } => Anchor::TopRight,
    };
    positioner.set_anchor(anchor);
    positioner.set_gravity(Gravity::BottomRight);
    let mut adjustment =
        ConstraintAdjustment::SlideX | ConstraintAdjustment::SlideY | ConstraintAdjustment::FlipX;
    if !matches!(node.placement, Placement::Submenu { .. }) {
        adjustment |= ConstraintAdjustment::FlipY;
    }
    positioner.set_constraint_adjustment(adjustment);
}

impl WindowPopup {
    /// 【原生弹层】【创建节点】用 frame、node、parent 和 grab 创建表面，其他参数提供协议资源；返回弹层或校验错误
    pub fn create(
        frame: PopupFrame<'_>,
        node: PopupNode,
        parent: Option<&ParentSurface>,
        layer: Option<&LayerSurface>,
        grab: (WlSeat, u32),
        compositor: &smithay_client_toolkit::compositor::CompositorState,
        shell: &smithay_client_toolkit::shell::xdg::XdgShell,
        viewporter: Option<&WpViewporter>,
        qh: &QueueHandle<NativeWindow>,
    ) -> AppResult<Self> {
        validate(&frame)?;
        if frame.id != node.id {
            return Err("popup frame does not match its tree node".into());
        }
        if viewporter.is_none() && (frame.scale < 1.0 || frame.scale.fract() != 0.0) {
            return Err("fractional popup scale requires viewporter".into());
        }
        // 1. 【原生弹层】【父链抓取】子菜单使用仍存活父菜单的同一活动抓取，不能建立独立兄弟会话
        let positioner = XdgPositioner::new(shell)?;
        positioner.set_size(frame.size.0 as i32, frame.size.1 as i32);
        position(&positioner, node, &frame);
        let popup = Popup::from_surface(
            parent,
            &positioner,
            qh,
            compositor.create_surface(qh),
            shell,
        )?;
        if let Some(layer) = layer {
            layer.get_popup(popup.xdg_popup());
        }
        popup.xdg_popup().grab(&grab.0, grab.1);
        let viewport = viewporter.map(|manager| {
            let viewport = manager.get_viewport(popup.wl_surface(), qh, SurfaceId::Main);
            viewport.set_destination(frame.size.0 as i32, frame.size.1 as i32);
            viewport
        });
        popup.wl_surface().commit();
        log::info!(
            "【原生弹层】【节点打开】id={}, parent={:?}, serial={}",
            frame.id,
            node.parent,
            grab.1
        );
        Ok(Self {
            id: frame.id,
            parent: node.parent,
            popup,
            keyboard_focused: false,
            scale: frame.scale,
            grab,
            configured: false,
            size: frame.size,
            pixel_size: frame.pixel_size,
            pixels: frame.pixels.to_vec(),
            buffers: Vec::new(),
            viewport,
            dirty: true,
            frame_pending: false,
        })
    }

    /// 【原生弹层】【像素更新】用 frame 替换当前脏像素并返回校验结果，活动抓取期间不接受编号或尺寸变化
    pub fn update(&mut self, frame: PopupFrame<'_>) -> AppResult {
        validate(&frame)?;
        if self.id != frame.id
            || self.size != frame.size
            || self.scale != frame.scale
            || self.pixel_size != frame.pixel_size
        {
            return Err("popup identity or dimensions changed during grab".into());
        }
        self.pixels.clear();
        self.pixels.extend_from_slice(frame.pixels);
        self.dirty = true;
        Ok(())
    }

    /// 【原生弹层】【配置确认】标记当前表面已接受配置，无参数、无返回值
    pub fn configure(&mut self) {
        self.configured = true;
    }

    /// 【原生弹层】【帧许可】取消当前帧等待，无参数、无返回值；旧表面的回调由父层隔离
    pub fn frame_done(&mut self) {
        self.frame_pending = false;
    }

    /// 【原生弹层】【有界绘制】通过 pool 和 qh 提交独立表面像素，返回缓冲错误，配置或帧许可不足时不空转
    pub fn paint(&mut self, pool: &mut SlotPool, qh: &QueueHandle<NativeWindow>) -> AppResult {
        if !self.configured || !self.dirty || self.frame_pending {
            return Ok(());
        }
        let Some(index) = buffers::available(pool, &mut self.buffers, self.pixel_size)? else {
            return Ok(());
        };
        let buffer = &self.buffers[index];
        let canvas = buffer.canvas(pool).ok_or("popup buffer remains active")?;
        crate::utils::copy_swizzled(canvas, &self.pixels);
        if self.viewport.is_none() {
            self.popup.wl_surface().set_buffer_scale(self.scale as i32);
        }
        buffer.attach_to(self.popup.wl_surface())?;
        self.popup.wl_surface().damage_buffer(
            0,
            0,
            self.pixel_size.0 as i32,
            self.pixel_size.1 as i32,
        );
        self.popup
            .wl_surface()
            .frame(qh, FrameCallbackData(self.popup.wl_surface().clone()));
        self.popup.wl_surface().commit();
        self.dirty = false;
        self.frame_pending = true;
        Ok(())
    }

    /// 【原生弹层】【表面引用】返回当前协议表面，无参数，不通过可复用数字编号判断身份
    pub fn surface(&self) -> &WlSurface {
        self.popup.wl_surface()
    }
}

#[cfg(test)]
mod position_tests {
    use super::*;

    /// 【原生弹层测试】【分数点边界】点锚定的整数单元必须包含原始指针，正负坐标均不能向右下方移动，无参数、无返回值
    #[test]
    fn fractional_point_anchor_contains_the_original_pointer() {
        for point in [
            (503.1914, 121.5391),
            (503.5078, 120.8984),
            (-0.2, -0.8),
            (20.0, 30.0),
        ] {
            let rect = anchor_rect(Placement::Point, point);
            assert!(
                rect.0 as f32 <= point.0 && point.0 < (rect.0 + 1) as f32,
                "x anchor excludes pointer {point:?}"
            );
            assert!(
                rect.1 as f32 <= point.1 && point.1 < (rect.1 + 1) as f32,
                "y anchor excludes pointer {point:?}"
            );
        }
    }
}
