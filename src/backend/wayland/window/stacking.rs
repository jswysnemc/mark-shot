//! 【原生窗口】【独立置顶】主窗口切换协议角色时保留内容、任务和逻辑尺寸，输入身份与协议资源重新建立

use super::{
    WindowAction, WindowEvent,
    layer_geometry::{Geometry, Gesture},
    role::Role,
    runtime::NativeWindow,
    surface::SurfaceId,
};
use smithay_client_toolkit::compositor::SurfaceData;
use wayland_client::{Proxy, QueueHandle, protocol::wl_output::WlOutput};

struct OutputPlacement {
    output: WlOutput,
    origin: (i32, i32),
    bounds: (i32, i32),
}

impl NativeWindow {
    /// 【原生窗口】【目标输出】返回主表面所在输出及逻辑边界，尚未进入输出时使用当前已绑定输出，无参数
    fn stacking_output(&self) -> Option<OutputPlacement> {
        let output = self
            .main
            .role
            .wl_surface()
            .data::<SurfaceData<()>>()
            .and_then(|data| data.outputs().next())
            .or_else(|| self.outputs.outputs().next())?;
        let info = self.outputs.info(&output)?;
        Some(OutputPlacement {
            output,
            origin: info.logical_position.unwrap_or_default(),
            bounds: info.logical_size?,
        })
    }

    /// 【原生窗口】【模式同步】按 qh 切换主窗口角色，保留业务对象；模态存活时延后切换，无返回值
    pub(super) fn sync_stacking(&mut self, qh: &QueueHandle<Self>) {
        let requested = self.content.always_on_top() && self.layers.is_some();
        let current = self.main.role.as_layer().is_some();
        if self.dialog.is_some() || self.exit {
            return;
        }
        if requested == current {
            if current
                && !self.layer_geometry.initialized
                && let Some(target) = self.stacking_output()
            {
                self.layer_geometry = self.layer_geometry.centered(target.origin, target.bounds);
                self.main.role.place(self.layer_geometry);
                self.main.role.commit();
            }
            return;
        }
        let output = self.stacking_output();
        let mut geometry = Geometry::new(self.main.size);
        if let Some(target) = &output {
            geometry = geometry.centered(target.origin, target.bounds);
        }
        // 1. 【原生窗口】【旧输入回收】角色切换取消抓取和协议目标，但不析构编辑器或翻译任务
        self.close_popups();
        self.dispatch_to(SurfaceId::Main, WindowEvent::Focused(false), None);
        if let Some(focus) = self.input_target.focus {
            self.leave_text_input(focus.id);
        }
        self.text_input.sync(None);
        self.pointer_grabs.cancel();
        self.keyboard_grabs.cancel();
        self.keyboard_surface = None;
        self.layer_gesture = None;
        let role = if requested {
            Role::layer(
                &self.main.spec,
                geometry,
                output.as_ref().map(|value| &value.output),
                &self.compositor,
                self.layers.as_ref().unwrap(),
                qh,
            )
        } else {
            Role::window(&self.main.spec, &self.compositor, &self.shell, qh)
        };
        // 2. 【原生窗口】【新角色提交】附属对象和缓冲重新绑定，首帧就绪通知不重复发送
        self.main.recreate(
            role,
            geometry.size,
            self.viewporter.as_ref(),
            self.fractional_manager.as_ref(),
            qh,
        );
        self.layer_geometry = geometry;
        self.main.role.commit();
        log::info!(
            "【原生窗口】【置顶切换】置顶={requested}，尺寸={:?}",
            geometry.size
        );
    }

    /// 【原生窗口】【浮层事务】按 action 创建主浮层移动或尺寸调整，普通窗口返回 false，不启动正文拖选
    pub(super) fn begin_layer_gesture(&mut self, action: WindowAction) -> bool {
        if self.main.role.as_layer().is_none() {
            return false;
        }
        self.layer_gesture = Some(Gesture::new(
            self.layer_geometry,
            self.pointer_position,
            action == WindowAction::Resize,
        ));
        true
    }

    /// 【原生窗口】【浮层指针】按 position 更新活动浮层几何，返回是否消费；尺寸变更通过同一 Resize 事件通知内容
    pub(super) fn layer_motion(&mut self, position: (f64, f64)) -> bool {
        let Some(gesture) = self.layer_gesture else {
            return false;
        };
        let geometry = gesture.update(self.layer_geometry, position, self.main.spec.minimum);
        // 1. 【原生窗口】【重复通知】几何未变不提交，避免表面移动通知再次触发同一移动
        if geometry == self.layer_geometry {
            return true;
        }
        let resize = geometry.size != self.main.size;
        self.layer_geometry = geometry;
        self.main.role.place(geometry);
        if resize {
            self.main.size = geometry.size;
            if let Some(viewport) = &self.main.viewport {
                viewport.set_destination(geometry.size.0 as i32, geometry.size.1 as i32);
            }
            self.dispatch_to(
                SurfaceId::Main,
                WindowEvent::Resize {
                    size: geometry.size,
                },
                None,
            );
        }
        self.main.role.commit();
        true
    }
}
