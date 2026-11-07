//! 【贴图】【左键手势】边缘缩放先于文字选择，移动与预览保持固定输入坐标

use super::runtime::{Pin, Shell};
use crate::features::pinned_image::{
    placement::Geometry,
    resize::{Edge, Resize},
};
use wayland_client::QueueHandle;
use wayland_protocols::{
    wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape,
    xdg::shell::client::xdg_toplevel::ResizeEdge,
};

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Move,
    Resize(Edge),
    NativeResize(Edge),
}

#[derive(Clone, Copy)]
pub(super) struct Gesture {
    pub kind: Kind,
    pub start: Geometry,
    pub pointer: (f64, f64),
    pub origin: (i32, i32),
}

impl Gesture {
    /// 【贴图】【全局指针】将 position 从固定输入表面换算为全局逻辑坐标，不读取桌面指针
    pub fn global(self, position: (f64, f64)) -> (f64, f64) {
        (
            position.0 + f64::from(self.origin.0),
            position.1 + f64::from(self.origin.1),
        )
    }
}

/// 【贴图】【缩放光标】由 edge 返回原版双向光标，角落不使用移动手形
fn edge_cursor(edge: Edge) -> Shape {
    match edge {
        Edge::Left | Edge::Right => Shape::EwResize,
        Edge::Top | Edge::Bottom => Shape::NsResize,
        Edge::TopLeft | Edge::BottomRight => Shape::NwseResize,
        Edge::TopRight | Edge::BottomLeft => Shape::NeswResize,
    }
}

/// 【贴图】【系统缩放方向】将 edge 转换为 xdg 原生缩放方向，窗口位置由合成器维护
fn native_edge(edge: Edge) -> ResizeEdge {
    match edge {
        Edge::Left => ResizeEdge::Left,
        Edge::Right => ResizeEdge::Right,
        Edge::Top => ResizeEdge::Top,
        Edge::Bottom => ResizeEdge::Bottom,
        Edge::TopLeft => ResizeEdge::TopLeft,
        Edge::TopRight => ResizeEdge::TopRight,
        Edge::BottomLeft => ResizeEdge::BottomLeft,
        Edge::BottomRight => ResizeEdge::BottomRight,
    }
}

impl Pin {
    /// 【贴图】【当前缩放边】按 position 命中可见窗口边缘；裁切边或贴屏边改为移动，不缩放被隐藏的边
    fn edge(&self, position: (f64, f64)) -> Option<Edge> {
        let edge = Edge::at(
            (self.placement.visible.width, self.placement.visible.height),
            position,
            f64::from(crate::config::get().pinned.border_width + 5.0).max(8.0),
        )?;
        if matches!(self.shell, Shell::Layer(_)) {
            let screens = self.screens();
            if let Some((_, screen)) = screens
                .iter()
                .find(|(output, _)| self.output.as_ref() == Some(output))
                && edge.embedded(self.geometry(), *screen)
            {
                return None;
            }
        }
        Some(edge)
    }

    /// 【贴图】【光标切换】按 position 和当前手势显示边缘、文字或拖动光标，不改变窗口几何
    pub fn cursor(&self, position: (f64, f64)) {
        let edge = self
            .drag
            .and_then(|drag| match drag.kind {
                Kind::Resize(edge) | Kind::NativeResize(edge) => Some(edge),
                _ => None,
            })
            .or_else(|| self.edge(position));
        let shape = if self.translation_task.is_some() {
            Shape::Wait
        } else if self
            .drag
            .is_some_and(|drag| matches!(drag.kind, Kind::Move))
        {
            Shape::Grabbing
        } else if let Some(edge) = edge {
            edge_cursor(edge)
        } else if self.text_selection && self.text.line_at(self.image_point(position)).is_some() {
            Shape::Text
        } else if self.drag.is_some() {
            Shape::Grabbing
        } else {
            Shape::Grab
        };
        if let Some(cursor) = &self.cursor {
            cursor.set_shape(self.serial, shape);
        }
    }

    /// 【贴图】【左键开始】按 position、serial 分流边缘缩放、文字选择、双击关闭和移动；只记录手势，不提前创建预览，无返回值
    pub fn begin_gesture(&mut self, position: (f64, f64), serial: u32) {
        if self.popup.take().is_some() {
            return;
        }
        let origin = self.input_origin();
        let pointer = (
            position.0 + f64::from(origin.0),
            position.1 + f64::from(origin.1),
        );
        let edge = self.edge(position);
        if edge.is_none() {
            let point = self.image_point(position);
            if self.text_selection && self.text.line_at(point).is_some() {
                if self
                    .clicks
                    .register(true, (pointer.0 as f32, pointer.1 as f32))
                {
                    if let Some(line) = self.text.line_at(point) {
                        self.text.select_line(line);
                    }
                } else {
                    self.text.begin_drag(point);
                }
                self.selecting = true;
                self.dirty = true;
                return;
            }
            if self
                .clicks
                .register(false, (pointer.0 as f32, pointer.1 as f32))
            {
                self.exit = true;
                return;
            }
        } else {
            self.clicks.clear();
        }
        self.text.deselect();
        self.dirty = true;
        let native = matches!(self.shell, Shell::Window(_));
        let kind = match edge {
            Some(edge) if native => Kind::NativeResize(edge),
            Some(edge) => Kind::Resize(edge),
            None => Kind::Move,
        };
        self.drag = Some(Gesture {
            kind,
            start: self.geometry(),
            pointer,
            origin,
        });
        self.native_resizing = false;
        if let Shell::Window(window) = &self.shell
            && let Some(seat) = &self.pointer_seat
        {
            if let Some(edge) = edge {
                window.resize(seat, serial, native_edge(edge));
            } else {
                window.move_(seat, serial);
            }
        }
        self.cursor(position);
    }

    /// 【贴图】【左键继续】由 position 更新文字或累计移动、缩放；qh 只更新预览，不移动隐式抓取表面
    pub fn continue_gesture(&mut self, position: (f64, f64), qh: &QueueHandle<Self>) {
        if self.selecting {
            self.text.extend_drag(self.image_point(position));
            self.dirty = true;
        } else if let Some(drag) = self.drag {
            let pointer = drag.global(position);
            if (pointer.0 - drag.pointer.0).hypot(pointer.1 - drag.pointer.1)
                > f64::from(crate::config::get().input.double_click_distance)
            {
                self.clicks.clear();
            }
            if matches!(self.shell, Shell::Layer(_)) {
                let geometry = match drag.kind {
                    Kind::Move => Geometry::new(
                        (
                            (f64::from(drag.start.left) + pointer.0 - drag.pointer.0).round()
                                as i32,
                            (f64::from(drag.start.top) + pointer.1 - drag.pointer.1).round() as i32,
                        ),
                        (drag.start.width, drag.start.height),
                    ),
                    Kind::Resize(edge) => Resize {
                        edge,
                        start: drag.start,
                        pointer: drag.pointer,
                    }
                    .geometry(pointer),
                    Kind::NativeResize(_) => return,
                };
                if geometry != self.geometry() {
                    self.position = (geometry.left, geometry.top);
                    self.image.set_size((geometry.width, geometry.height));
                    self.resize(false, qh);
                }
            }
        }
        self.cursor(position);
    }

    /// 【贴图】【左键结束】完成拖动并重建目标输出；qh 用于交还图像，预览在主表面提交后才释放
    pub fn finish_gesture(&mut self, qh: &QueueHandle<Self>) {
        let dragged = self.drag.take().is_some();
        self.selecting = false;
        // 1. 【贴图】【无位移点击】没有实际预览的点击只结束抓取，不重配 layer 或撤销键盘焦点
        if dragged && (matches!(self.shell, Shell::Window(_)) || self.preview.is_some()) {
            self.resize(false, qh);
        }
    }

    /// 【贴图】【系统尺寸反馈】按 configure 的 requested 尺寸和缩放方向保留比例；返回是否处理原生缩放
    pub fn native_size(
        &mut self,
        requested: (u32, u32),
        resizing: bool,
        qh: &QueueHandle<Self>,
    ) -> bool {
        let Some(drag) = self.drag else {
            return false;
        };
        let Kind::NativeResize(edge) = drag.kind else {
            return false;
        };
        if !resizing && !self.native_resizing {
            return false;
        }
        self.native_resizing = resizing;
        let (x, y) = edge.axes();
        let pointer = (
            drag.pointer.0 + f64::from(x) * (f64::from(requested.0) - f64::from(drag.start.width)),
            drag.pointer.1 + f64::from(y) * (f64::from(requested.1) - f64::from(drag.start.height)),
        );
        let geometry = Resize {
            edge,
            start: drag.start,
            pointer: drag.pointer,
        }
        .geometry(pointer);
        self.image.set_size((geometry.width, geometry.height));
        self.resize(false, qh);
        if !resizing {
            self.drag = None;
        }
        true
    }
}
