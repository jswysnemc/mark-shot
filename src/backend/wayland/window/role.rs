//! 【原生窗口】【协议角色】xdg 普通窗口和 layer-shell 置顶窗口共用表面接口，不使用系统命令伪造置顶

use super::{WindowSpec, layer_geometry::Geometry, runtime::NativeWindow};
use smithay_client_toolkit::{
    compositor::{CompositorState, Region},
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface},
        xdg::{
            XdgShell,
            window::{Window, WindowDecorations},
        },
    },
};
use wayland_client::{
    QueueHandle,
    protocol::{wl_output::WlOutput, wl_surface::WlSurface},
};

pub(super) enum Role {
    Window(Window),
    Layer(LayerSurface),
}

impl Role {
    /// 【原生窗口】【普通角色】按 spec 和当前协议资源创建 xdg 窗口，返回未提交角色
    pub fn window(
        spec: &WindowSpec,
        compositor: &CompositorState,
        shell: &XdgShell,
        qh: &QueueHandle<NativeWindow>,
    ) -> Self {
        let window = shell.create_window(
            compositor.create_surface(qh),
            WindowDecorations::ClientOnly,
            qh,
        );
        window.set_title(spec.title.clone());
        window.set_app_id(spec.app_id.clone());
        window.set_min_size(Some(spec.minimum));
        Self::Window(window)
    }

    /// 【原生窗口】【置顶角色】按 spec、geometry 和 output 创建真正 Top 层角色，返回未提交角色；无输出几何时由合成器居中
    pub fn layer(
        spec: &WindowSpec,
        geometry: Geometry,
        output: Option<&WlOutput>,
        compositor: &CompositorState,
        layers: &LayerShell,
        qh: &QueueHandle<NativeWindow>,
    ) -> Self {
        let layer = layers.create_layer_surface(
            qh,
            compositor.create_surface(qh),
            Layer::Top,
            Some(spec.app_id.clone()),
            output,
        );
        layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        layer.set_exclusive_zone(-1);
        let role = Self::Layer(layer);
        role.place(geometry);
        role
    }

    /// 【原生窗口】【表面引用】返回当前角色的协议表面，不复制缓冲，无参数
    pub fn wl_surface(&self) -> &WlSurface {
        match self {
            Self::Window(window) => window.wl_surface(),
            Self::Layer(layer) => layer.wl_surface(),
        }
    }

    /// 【原生窗口】【普通窗口引用】返回普通角色的窗口，置顶浮层没有 xdg-toplevel，无参数
    pub fn as_window(&self) -> Option<&Window> {
        match self {
            Self::Window(window) => Some(window),
            Self::Layer(_) => None,
        }
    }

    /// 【原生窗口】【浮层引用】返回 layer-shell 角色，普通窗口返回空值，无参数
    pub fn as_layer(&self) -> Option<&LayerSurface> {
        match self {
            Self::Layer(layer) => Some(layer),
            Self::Window(_) => None,
        }
    }

    /// 【原生窗口】【角色提交】提交当前协议表面，无参数、无返回值
    pub fn commit(&self) {
        self.wl_surface().commit();
    }

    /// 【原生窗口】【浮层定位】按 geometry 设置输出本地锚定、边距及尺寸，不改变角色身份，无返回值
    pub fn place(&self, geometry: Geometry) {
        if let Self::Layer(layer) = self {
            layer.set_anchor(if geometry.initialized {
                Anchor::TOP | Anchor::LEFT
            } else {
                Anchor::empty()
            });
            let (left, top) = geometry.margin();
            layer.set_margin(top, 0, 0, left);
            layer.set_size(geometry.size.0, geometry.size.1);
        }
    }

    /// 【原生窗口】【模态层级】按 active 暂时降低浮层并隔离输入，compositor 提供空区域；返回分配结果，不修改用户置顶配置
    pub fn modal(&self, active: bool, compositor: &CompositorState) -> crate::AppResult {
        if let Self::Layer(layer) = self {
            let region = active.then(|| Region::new(compositor)).transpose()?;
            self.wl_surface()
                .set_input_region(region.as_ref().map(Region::wl_region));
            layer.set_layer(if active { Layer::Bottom } else { Layer::Top });
            layer.set_keyboard_interactivity(if active {
                KeyboardInteractivity::None
            } else {
                KeyboardInteractivity::OnDemand
            });
            self.commit();
        }
        Ok(())
    }
}
