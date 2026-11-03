//! 【贴图】【几何同步】将全局逻辑几何映射到输出内表面，拖动时只更新预览

use super::runtime::{Pin, Shell};
use crate::features::pinned_image::placement::{self, Geometry, Placement};
use wayland_client::{QueueHandle, protocol::wl_output};

impl Pin {
    /// 【贴图】【输出快照】返回有完整逻辑坐标的输出与矩形，不使用捕获或额外线程
    pub fn screens(&self) -> Vec<(wl_output::WlOutput, Geometry)> {
        self.outputs
            .outputs()
            .filter_map(|output| {
                let info = self.outputs.info(&output)?;
                let (width, height) = info.logical_size?;
                let origin = info.logical_position.unwrap_or_default();
                (width > 0 && height > 0)
                    .then_some((output, Geometry::new(origin, (width as u32, height as u32))))
            })
            .collect()
    }

    /// 【贴图】【逻辑矩形】返回当前完整图片几何，屏幕外部分仍保留原图比例
    pub fn geometry(&self) -> Geometry {
        Geometry::new(self.position, self.size)
    }

    /// 【贴图】【输入原点】返回当前输入表面的全局原点；拖动期间原表面固定，不跟随预览
    pub fn input_origin(&self) -> (i32, i32) {
        (self.placement.visible.left, self.placement.visible.top)
    }

    /// 【贴图】【显示坐标】将 position 从可见表面转换为完整图像本地坐标，兼容左上裁切
    pub fn local_point(&self, position: (f64, f64)) -> (f64, f64) {
        let origin = self.input_origin();
        (
            position.0 + f64::from(origin.0) - f64::from(self.position.0),
            position.1 + f64::from(origin.1) - f64::from(self.position.1),
        )
    }

    /// 【贴图】【尺寸更新】按模型更新完整 size，center 保持中心；qh 用于输出切换，拖动时不更换输入表面
    pub fn resize(&mut self, center: bool, qh: &QueueHandle<Self>) {
        let size = self.image.size();
        if center {
            self.position.0 = (f64::from(self.position.0)
                + (f64::from(self.size.0) - f64::from(size.0)) / 2.0)
                .round() as i32;
            self.position.1 = (f64::from(self.position.1)
                + (f64::from(self.size.1) - f64::from(size.1)) / 2.0)
                .round() as i32;
        }
        self.size = size;
        if matches!(self.shell, Shell::Layer(_)) {
            let screens = self.screens();
            let Some(index) = placement::best_output(
                self.geometry(),
                &screens.iter().map(|item| item.1).collect::<Vec<_>>(),
            ) else {
                self.shell.surface().commit();
                return;
            };
            let (output, screen) = &screens[index];
            let placement = placement::place(self.geometry(), *screen);
            self.position = (placement.logical.left, placement.logical.top);
            if self.drag.is_some() {
                if let Err(error) = self.update_preview(output, placement, qh) {
                    self.report(Err(error));
                }
                return;
            }
            self.placement = placement;
            self.bounds = (screen.width, screen.height);
            if self.output.as_ref() != Some(output) {
                self.output = Some(output.clone());
                self.recreate(qh);
            }
            if let Shell::Layer(layer) = &self.shell {
                layer.set_size(placement.visible.width, placement.visible.height);
                layer.set_margin(placement.margin.1, 0, 0, placement.margin.0);
            }
        } else {
            // 1. 【贴图】【普通窗口】绝对位置交给合成器，仅提交完整客户端几何
            self.placement = Placement {
                logical: self.geometry(),
                visible: self.geometry(),
                offset: (0, 0),
                margin: (0, 0),
            };
        }
        self.dirty = true;
        // 2. 【贴图】【原子尺寸】普通窗口的几何必须随新缓冲提交，不能向合成器暴露旧缓冲的新尺寸
        if matches!(self.shell, Shell::Layer(_)) || !self.configured {
            self.shell.surface().commit();
        }
    }
}
