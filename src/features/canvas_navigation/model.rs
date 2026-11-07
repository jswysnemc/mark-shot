//! 【画布导航】【视图模型】缩放围绕窗口锚点，平移从初始中心求值，原图和标注保持独立

use super::coordinates::{Point, Projection, Rect, Size, finite};

pub const MIN_ZOOM: f64 = 0.25;
pub const MAX_ZOOM: f64 = 64.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Navigation {
    image_size: Size,
    window_size: Size,
    zoom: f64,
    center: Option<Point>,
    rect: Rect,
}

impl Navigation {
    /// 【画布导航】【初始构造】按image_size与window_size创建适配窗口的视图，available决定是否初始化中心，无效尺寸返回空值
    pub fn new(image_size: Size, window_size: Size, available: bool) -> Option<Self> {
        if image_size.0 == 0 || image_size.1 == 0 || window_size.0 == 0 || window_size.1 == 0 {
            return None;
        }
        let mut view = Self {
            image_size,
            window_size,
            zoom: 1.0,
            center: None,
            rect: Rect::default(),
        };
        view.refresh(available);
        Some(view)
    }

    /// 【画布导航】【投影读取】返回当前原图尺寸与窗口矩形，不复制像素，无参数
    pub fn projection(self) -> Projection {
        Projection {
            image_size: self.image_size,
            widget_rect: self.rect,
        }
    }

    /// 【画布导航】【窗口读取】返回当前窗口逻辑尺寸，无参数
    pub fn window_size(self) -> Size {
        self.window_size
    }

    /// 【画布导航】【缩放读取】返回相对于适配窗口尺寸的缩放倍率，无参数
    pub fn zoom(self) -> f64 {
        self.zoom
    }

    /// 【画布导航】【中心读取】返回已初始化的原图中心，选择阶段可以为空，无参数
    pub fn center(self) -> Option<Point> {
        self.center
    }

    /// 【画布导航】【适配比例】返回原图等比适配窗口时的逻辑缩放比例，无参数
    fn fit_scale(self) -> f64 {
        (f64::from(self.window_size.0) / f64::from(self.image_size.0))
            .min(f64::from(self.window_size.1) / f64::from(self.image_size.1))
    }

    /// 【画布导航】【原图中心】返回整幅原图的浮点中心，无参数
    fn image_center(self) -> Point {
        (
            f64::from(self.image_size.0) / 2.0,
            f64::from(self.image_size.1) / 2.0,
        )
    }

    /// 【画布导航】【视图约束】按available重算窗口矩形，超出窗口的轴限制可见中心，能完整显示的轴重新居中，无返回值
    pub fn refresh(&mut self, available: bool) {
        let fit = self.fit_scale();
        let window = (f64::from(self.window_size.0), f64::from(self.window_size.1));
        if !available {
            let size = (
                f64::from(self.image_size.0) * fit,
                f64::from(self.image_size.1) * fit,
            );
            self.rect = Rect {
                x: (window.0 - size.0) / 2.0,
                y: (window.1 - size.1) / 2.0,
                width: size.0,
                height: size.1,
            };
            return;
        }
        let scale = fit * self.zoom;
        let image = (f64::from(self.image_size.0), f64::from(self.image_size.1));
        let size = (image.0 * scale, image.1 * scale);
        let center = self.center.unwrap_or_else(|| self.image_center());
        let axis = |value: f64, length: f64, scaled: f64, visible: f64| {
            if scaled <= visible {
                length / 2.0
            } else {
                let half = visible / (2.0 * scale);
                value.clamp(half, length - half)
            }
        };
        let center = (
            axis(center.0, image.0, size.0, window.0),
            axis(center.1, image.1, size.1, window.1),
        );
        self.center = Some(center);
        self.rect = Rect {
            x: window.0 / 2.0 - center.0 * scale,
            y: window.1 / 2.0 - center.1 * scale,
            width: size.0,
            height: size.1,
        };
    }

    /// 【画布导航】【窗口变化】按window_size与available重算视图，返回是否变化；零尺寸不改变现有状态
    pub fn resize(&mut self, window_size: Size, available: bool) -> bool {
        if window_size.0 == 0 || window_size.1 == 0 {
            return false;
        }
        let before = *self;
        self.window_size = window_size;
        self.refresh(available);
        *self != before
    }

    /// 【画布导航】【锚点缩放】按factor与widget_anchor调整缩放，available控制归属，返回视图是否变化；图外锚点使用已存原图中心
    pub fn zoom_at(&mut self, factor: f64, widget_anchor: Point, available: bool) -> bool {
        if !available || factor <= 0.0 || factor.is_nan() || !finite(widget_anchor) {
            return false;
        }
        let before = *self;
        let anchor = if self.rect.contains(widget_anchor) {
            self.projection().widget_to_image(widget_anchor).unwrap()
        } else {
            self.center.unwrap_or_else(|| self.image_center())
        };
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let scale = self.fit_scale() * self.zoom;
        self.center = Some((
            anchor.0 - (widget_anchor.0 - f64::from(self.window_size.0) / 2.0) / scale,
            anchor.1 - (widget_anchor.1 - f64::from(self.window_size.1) / 2.0) / scale,
        ));
        self.refresh(available);
        *self != before
    }

    /// 【画布导航】【完整平移】按start_center、start_widget与current_widget重算完整位移，available控制归属，返回实际变化，不累计前帧误差
    pub fn pan_from(
        &mut self,
        start_center: Point,
        start_widget: Point,
        current_widget: Point,
        available: bool,
    ) -> bool {
        if !available || !finite(start_center) || !finite(start_widget) || !finite(current_widget) {
            return false;
        }
        let before = *self;
        let scale = self.rect.width / f64::from(self.image_size.0);
        self.center = Some((
            start_center.0 - (current_widget.0 - start_widget.0) / scale,
            start_center.1 - (current_widget.1 - start_widget.1) / scale,
        ));
        self.refresh(available);
        *self != before
    }

    /// 【画布导航】【缩放复位】按available恢复适配尺寸和原图中心，返回实际变化，不保存标注历史
    pub fn reset(&mut self, available: bool) -> bool {
        let before = *self;
        self.zoom = 1.0;
        self.center = Some(self.image_center());
        self.refresh(available);
        *self != before
    }

    /// 【画布导航】【滚动中心】按values与active_axes把整数滚动值映射回原图中心，available控制归属，返回实际变化
    pub fn center_from_scrollbars(
        &mut self,
        values: (i32, i32),
        active_axes: (bool, bool),
        available: bool,
    ) -> bool {
        if !available {
            return false;
        }
        let before = *self;
        let scale = self.rect.width / f64::from(self.image_size.0);
        self.center = Some((
            if active_axes.0 {
                (f64::from(values.0) + f64::from(self.window_size.0) / 2.0) / scale
            } else {
                self.image_center().0
            },
            if active_axes.1 {
                (f64::from(values.1) + f64::from(self.window_size.1) / 2.0) / scale
            } else {
                self.image_center().1
            },
        ));
        self.refresh(available);
        *self != before
    }
}
