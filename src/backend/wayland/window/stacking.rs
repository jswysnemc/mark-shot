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

pub(super) struct OutputPlacement {
    pub output: WlOutput,
    pub origin: (i32, i32),
    pub bounds: (i32, i32),
}

impl NativeWindow {
    /// 【原生窗口】【目标输出】返回主表面所在输出及逻辑边界，尚未进入输出时使用当前已绑定输出，无参数
    pub(super) fn stacking_output(&self) -> Option<OutputPlacement> {
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

    /// 【原生窗口】【模式同步】按 qh 切换主窗口角色或落地跨屏浮层，保留业务对象；模态存活时延后切换，无返回值
    pub(super) fn sync_stacking(&mut self, qh: &QueueHandle<Self>) {
        let requested = self.content.always_on_top() && self.layers.is_some();
        let current = self.main.role.as_layer().is_some();
        if self.dialog.is_some() || self.exit {
            return;
        }
        if requested && current && self.layer_rebind_needed {
            self.layer_rebind_needed = false;
            if let Some((output, origin)) = self.output_for_geometry(self.layer_geometry)
                && let Some(layers) = &self.layers
            {
                let mut geometry = self.layer_geometry;
                geometry.output_origin = origin;
                geometry.initialized = true;
                let role = Role::layer(
                    &self.main.spec,
                    geometry,
                    Some(&output),
                    self.content.above_pinned_windows(),
                    &self.compositor,
                    layers,
                    qh,
                );
                self.main.recreate(
                    role,
                    geometry.size,
                    self.viewporter.as_ref(),
                    self.fractional_manager.as_ref(),
                    qh,
                );
                self.layer_geometry = geometry;
                self.main.role.commit();
            }
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
        self.layer_preview = None;
        self.layer_rebind_needed = false;
        let role = if requested {
            Role::layer(
                &self.main.spec,
                geometry,
                output.as_ref().map(|value| &value.output),
                self.content.above_pinned_windows(),
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

    /// 【原生窗口】【浮层指针】按 position 和 qh 更新活动浮层几何，移动期间固定主表面并驱动预览层，返回是否消费
    pub(super) fn layer_motion(&mut self, position: (f64, f64), qh: &QueueHandle<Self>) -> bool {
        let Some(gesture) = self.layer_gesture else {
            return false;
        };
        let geometry = gesture.update(self.layer_geometry, position, self.main.spec.minimum);
        // 1. 【原生窗口】【重复通知】几何未变不提交，避免表面重复刷新
        if geometry == self.layer_geometry {
            return true;
        }
        self.layer_geometry = geometry;
        if gesture.resizing() {
            let resize = geometry.size != self.main.size;
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
            self.main.dirty = true;
            if !self.main.frame_pending {
                self.main.role.commit();
            }
            return true;
        }
        // 2. 【原生窗口】【静止抓取】拖动期间不移动主浮层，仅更新无输入预览浮层以保持指针坐标原点恒定
        if let Err(error) = self.update_layer_preview(geometry, qh) {
            log::error!("【原生窗口】【拖动预览】更新失败: {error}");
        }
        true
    }

    /// 【原生窗口】【浮层收尾】结束当前浮层拖动或缩放事务，把最终位置原子同步回主浮层，无参数、无返回值
    pub(super) fn end_layer_gesture(&mut self) {
        let Some(gesture) = self.layer_gesture.take() else {
            return;
        };
        if gesture.resizing() {
            return;
        }
        if let Some((_, origin)) = self.output_for_geometry(self.layer_geometry)
            && origin != self.layer_geometry.output_origin
        {
            self.layer_rebind_needed = true;
            return;
        }
        self.main.role.place(self.layer_geometry);
        if self.main.hidden_for_drag {
            self.main.hidden_for_drag = false;
            self.main.dirty = true;
        } else {
            self.layer_preview = None;
            self.main.role.commit();
        }
    }
}
