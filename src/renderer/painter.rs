// Painter is a wrapper over pixmap plus drawing context
// viewport (origin and scale), icons, and fonts.
//
// Shape helpers (fill_path, fill_rounded, etc) automatically apply viewport
// transform.
// (draw_pixmap() and canvas() draw directly in raw pixels without it)
use std::collections::HashMap;

use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::{
    BlendMode, Color, FillRule, FilterQuality, Paint, Path, PathBuilder, Pixmap, PixmapPaint, Rect,
    Stroke, Transform,
};
use usvg::Tree;

use super::paths::rounded_rect_path;
use crate::theme::radius;

pub type Icons = HashMap<&'static str, Tree>;

pub(super) struct Fonts<'a> {
    pub(super) system: &'a mut FontSystem,
    pub(super) cache: &'a mut SwashCache,
}

pub struct Painter<'a> {
    pub(super) canvas: &'a mut Pixmap,
    scale: f32,
    chrome_scale: f32,
    scene_transform: Transform,
    // top-left corner position in the coorindates callers drawn in
    pub(super) origin: (f32, f32),
    pub(super) icons: Option<&'a Icons>,
    pub(super) fonts: Option<Fonts<'a>>,
    drawing_bounds: Option<Rect>,
    ruler_projection: crate::editor::canvas_navigation::transform::ViewTransform,
    frozen_image: Option<super::FrozenImage<'a>>,
    pixel_phase: Option<(f64, f64)>,
    pub(super) text_caret_visible: bool,
}

impl<'a> Painter<'a> {
    pub fn new(canvas: &'a mut Pixmap) -> Self {
        Self {
            canvas,
            scale: 1.0,
            chrome_scale: 1.0,
            scene_transform: Transform::identity(),
            origin: (0.0, 0.0),
            icons: None,
            fonts: None,
            drawing_bounds: None,
            ruler_projection: Default::default(),
            frozen_image: None,
            pixel_phase: None,
            text_caret_visible: true,
        }
    }

    pub fn with_icons(mut self, icons: &'a Icons) -> Self {
        self.icons = Some(icons);
        self
    }

    pub fn with_text(mut self, system: &'a mut FontSystem, cache: &'a mut SwashCache) -> Self {
        self.fonts = Some(Fonts { system, cache });
        self
    }

    pub fn with_origin(mut self, origin: (f32, f32)) -> Self {
        self.origin = origin;
        self
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        if self.scale != scale {
            self.pixel_phase = None;
        }
        self.scale = scale;
        self
    }

    /// 【文字绘制】【光标相位】将 visible 交给标注绘制并返回上下文；只改变光标，不隐藏正文或字符选区
    pub fn with_text_caret(mut self, visible: bool) -> Self {
        self.text_caret_visible = visible;
        self
    }

    /// 【渲染】【绘图边界】设置 bounds 为全局标注边界；返回绘制器，局部缓冲继承该边界
    pub fn with_drawing_bounds(mut self, bounds: Option<Rect>) -> Self {
        self.drawing_bounds = bounds;
        self
    }

    /// 【渲染】【边界读取】返回显式全局绘图边界，不把局部损坏缓冲尺寸当作画布范围
    pub fn drawing_bounds(&self) -> Option<Rect> {
        self.drawing_bounds
    }

    /// 【尺子渲染】【窗口投影】按projection记录当前完整窗口矩阵，返回绘制器，局部缓冲原点不改变布局
    pub(crate) fn with_ruler_projection(
        mut self,
        projection: crate::editor::canvas_navigation::transform::ViewTransform,
    ) -> Self {
        self.ruler_projection = projection;
        self
    }

    /// 【尺子渲染】【投影读取】无参数，返回尺子布局使用的完整窗口投影
    pub(crate) fn ruler_projection(
        &self,
    ) -> crate::editor::canvas_navigation::transform::ViewTransform {
        self.ruler_projection
    }

    /// 【渲染】【冻结图像】设置 image 为持久图像标注的零复制采样来源；返回绘制器，不使用当前画布作为原图
    pub fn with_frozen_image(mut self, image: super::FrozenImage<'a>) -> Self {
        self.frozen_image = Some(image);
        self
    }

    /// 【渲染】【原图读取】返回当前冻结图像视图，供双框放大镜在绘制前读取像素
    pub fn frozen_image(&self) -> Option<super::FrozenImage<'a>> {
        self.frozen_image
    }

    pub fn canvas(&mut self) -> &mut Pixmap {
        self.canvas
    }

    /// 【画布导航】【控件比例】按unit设置屏幕控件的反向比例，返回绘制器，不修改正文几何
    pub(crate) fn with_chrome_scale(mut self, unit: f32) -> Self {
        self.chrome_scale = unit;
        self
    }

    /// 【标注样式】【窗口上下文】按window设置当前坐标相对窗口逻辑像素的比例，返回绘制器，设备像素比例仍由with_scale独立提供
    pub fn with_window_units(
        mut self,
        window: crate::features::annotation_style::window::WindowUnits,
    ) -> Self {
        self.chrome_scale = window.canvas(1.0);
        self
    }

    /// 【画布导航】【控件单位】无参数，返回抵消持续视图缩放的逻辑单位
    pub(crate) fn chrome_scale(&self) -> f32 {
        self.chrome_scale
    }

    /// 【标注样式】【绘制单位】无参数，返回抵消持续导航倍率的窗口单位，设备像素比例不参与窗口限制
    pub(crate) fn window_units(&self) -> crate::features::annotation_style::window::WindowUnits {
        crate::features::annotation_style::window::WindowUnits::new(self.chrome_scale)
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// 【渲染】【画布原点】返回当前画布左上角对应的逻辑坐标，供固定像素分块定位
    pub fn origin(&self) -> (f32, f32) {
        self.origin
    }

    /// 【渲染】【输出像素相位】返回原始画布像素网格的小数原点，子缓冲继承相位，不把非整数导出原点吸附到全局整数
    pub fn pixel_phase(&self) -> (f64, f64) {
        self.pixel_phase.unwrap_or_else(|| {
            let phase = |origin: f32| {
                let pixel = f64::from(origin) * f64::from(self.scale);
                if !pixel.is_finite() {
                    return 0.0;
                }
                let fraction = pixel.rem_euclid(1.0);
                if fraction < 0.0001 || fraction > 0.9999 {
                    0.0
                } else {
                    fraction
                }
            };
            (phase(self.origin.0), phase(self.origin.1))
        })
    }

    pub fn size(&self) -> (f32, f32) {
        (
            self.canvas.width() as f32 / self.scale,
            self.canvas.height() as f32 / self.scale,
        )
    }

    pub fn to_px(&self, x: f32, y: f32) -> (f32, f32) {
        (
            (x - self.origin.0) * self.scale,
            (y - self.origin.1) * self.scale,
        )
    }

    /// 【标注绘制】【完整矩阵】返回场景变换、设备比例及画布偏移组成的矩阵，无参数，不改变状态
    pub fn transform(&self) -> Transform {
        if self.scene_transform.is_identity() {
            return Transform::from_translate(-self.origin.0, -self.origin.1)
                .post_scale(self.scale, self.scale);
        }
        self.transformed_viewport(self.scene_transform)
    }

    /// 【标注绘制】【偏移矩阵】按 dx、dy 返回先平移再执行场景变换的矩阵，供旋转正文保持阴影方向
    pub fn transform_shifted(&self, dx: f32, dy: f32) -> Transform {
        if self.scene_transform.is_identity() {
            return Transform::from_translate(-self.origin.0 + dx, -self.origin.1 + dy)
                .post_scale(self.scale, self.scale);
        }
        self.transformed_viewport(
            Transform::from_translate(dx, dy).post_concat(self.scene_transform),
        )
    }

    /// 【标注绘制】【固定视口】按 scene 和原始像素相位返回完整矩阵，局部缓冲原点吸附同一物理网格
    fn transformed_viewport(&self, scene: Transform) -> Transform {
        let phase = self.pixel_phase();
        let scale = f64::from(self.scale);
        let origin = |value: f32, phase: f64| (f64::from(value) * scale - phase).round() + phase;
        Transform::from_row(
            scene.sx * self.scale,
            scene.ky * self.scale,
            scene.kx * self.scale,
            scene.sy * self.scale,
            (f64::from(scene.tx) * scale - origin(self.origin.0, phase.0)) as f32,
            (f64::from(scene.ty) * scale - origin(self.origin.1, phase.1)) as f32,
        )
    }

    /// 【标注绘制】【局部变换】在 scene 坐标变换中执行 draw，结束后恢复父变换，无返回值，不修改画布或字体比例
    pub(crate) fn transformed(&mut self, scene: Transform, draw: impl FnOnce(&mut Painter<'_>)) {
        let previous = self.scene_transform;
        self.scene_transform = scene.post_concat(previous);
        draw(self);
        self.scene_transform = previous;
    }

    /// paints into another pixmap with the same stuff (icons, fonts, and scale)
    /// keep in mind, origin get reset; new canvas has its own coordinates
    /// use .with_origin afterwards if the new canvas is offset from it
    pub fn sub<'b>(&'b mut self, canvas: &'b mut Pixmap) -> Painter<'b> {
        let pixel_phase = Some(self.pixel_phase());
        Painter {
            canvas,
            scale: self.scale,
            chrome_scale: self.chrome_scale,
            scene_transform: self.scene_transform,
            origin: (0.0, 0.0),
            icons: self.icons,
            drawing_bounds: self.drawing_bounds,
            ruler_projection: self.ruler_projection,
            frozen_image: self.frozen_image,
            pixel_phase,
            text_caret_visible: self.text_caret_visible,
            fonts: self.fonts.as_mut().map(|fonts| Fonts {
                system: &mut *fonts.system,
                cache: &mut *fonts.cache,
            }),
        }
    }

    pub fn fill_path(&mut self, path: &Path, color: impl Into<Color>) {
        let mut paint = Paint::default();
        paint.set_color(color.into());
        paint.anti_alias = true;
        let transform = self.transform();
        self.canvas
            .fill_path(path, &paint, FillRule::Winding, transform, None);
    }

    pub fn stroke_path(&mut self, path: &Path, color: impl Into<Color>, stroke: &Stroke) {
        let mut paint = Paint::default();
        paint.set_color(color.into());
        paint.anti_alias = true;
        let transform = self.transform();
        self.canvas
            .stroke_path(path, &paint, stroke, transform, None);
    }

    pub fn fill_rounded(&mut self, rect: Rect, radius: f32, color: impl Into<Color>) {
        if let Some(path) = rounded_rect_path(&rect, radius, true, true, true, true) {
            self.fill_path(&path, color);
        }
    }

    /// 【界面渲染】【渐变按钮】在 rect 内以 radius 绘制从 top 到 bottom 的纵向渐变；继承当前比例与原点，无返回值
    pub fn fill_rounded_gradient(&mut self, rect: Rect, radius: f32, top: Color, bottom: Color) {
        let Some(path) = rounded_rect_path(&rect, radius, true, true, true, true) else {
            return;
        };
        let Some(shader) = tiny_skia::LinearGradient::new(
            tiny_skia::Point::from_xy(rect.left(), rect.top()),
            tiny_skia::Point::from_xy(rect.left(), rect.bottom()),
            vec![
                tiny_skia::GradientStop::new(0.0, top),
                tiny_skia::GradientStop::new(1.0, bottom),
            ],
            tiny_skia::SpreadMode::Pad,
            Transform::identity(),
        ) else {
            return;
        };
        let paint = Paint {
            shader,
            anti_alias: true,
            ..Paint::default()
        };
        let transform = self.transform();
        self.canvas
            .fill_path(&path, &paint, FillRule::Winding, transform, None);
    }

    pub fn fill_circle(&mut self, cx: f32, cy: f32, r: f32, color: impl Into<Color>) {
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy, r);
        if let Some(path) = pb.finish() {
            self.fill_path(&path, color);
        }
    }

    /// ignores origin & scale
    pub fn draw_pixmap(&mut self, x: i32, y: i32, pixmap: &Pixmap) {
        self.canvas.draw_pixmap(
            x,
            y,
            pixmap.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }

    /// uses sub, since to correclty apply opacity in draws them
    /// in a separate pixmap, and then apply on it as a whole
    /// otherwise icons wouldn't be transparent
    pub fn panel(
        &mut self,
        cache: &mut Option<Pixmap>,
        pos: (f32, f32),
        size: (f32, f32),
        opacity: f32,
        redraw: bool,
        content: impl FnOnce(&mut Painter),
    ) {
        self.panel_content(cache, pos, size, opacity, redraw, true, content);
    }

    /// 【界面渲染】【无框缓存】按 pos、size 绘制 cache；opacity 与 redraw 控制合成，不附加旧主题边框
    pub fn panel_unframed(
        &mut self,
        cache: &mut Option<Pixmap>,
        pos: (f32, f32),
        size: (f32, f32),
        opacity: f32,
        redraw: bool,
        content: impl FnOnce(&mut Painter),
    ) {
        self.panel_content(cache, pos, size, opacity, redraw, false, content);
    }

    /// 【界面渲染】【面板缓存】按参数更新并合成 cache；framed 决定边框，content 只在需要重绘时调用
    fn panel_content(
        &mut self,
        cache: &mut Option<Pixmap>,
        pos: (f32, f32),
        size: (f32, f32),
        opacity: f32,
        redraw: bool,
        framed: bool,
        content: impl FnOnce(&mut Painter),
    ) {
        let s = self.scale;
        let (px, py) = self.to_px(pos.0, pos.1);
        // rounding is obligatory to avoid desync and visual glitches
        let x = px.round();
        let y = py.round();
        let (w, h) = size;
        let (pw, ph) = ((w * s).ceil() as u32, (h * s).ceil() as u32);

        let fresh = cache
            .as_ref()
            .is_none_or(|p| p.width() != pw || p.height() != ph);
        if fresh {
            *cache = Pixmap::new(pw, ph);
        }
        let Some(pixmap) = cache.as_mut() else {
            return;
        };

        if fresh || redraw {
            pixmap.fill(Color::TRANSPARENT);
            content(&mut self.sub(pixmap));
        }

        self.canvas.draw_pixmap(
            x as i32,
            y as i32,
            pixmap.as_ref(),
            &PixmapPaint {
                opacity,
                blend_mode: BlendMode::SourceOver,
                quality: FilterQuality::Nearest,
            },
            Transform::identity(),
            None,
        );

        // (border follows where the pixmap went after rounding)
        let (bx, by) = (x / s + self.origin.0, y / s + self.origin.1);
        if framed {
            self.panel_border(bx, by, w, h, radius::panel(), opacity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【界面渲染测试】【渐变比例】两倍比例下颜色与圆角区域共同缩放，区域外保持透明
    #[test]
    fn rounded_gradient_uses_the_current_scale() {
        let mut canvas = Pixmap::new(32, 32).unwrap();
        Painter::new(&mut canvas)
            .with_scale(2.0)
            .fill_rounded_gradient(
                Rect::from_xywh(2.0, 3.0, 10.0, 10.0).unwrap(),
                2.0,
                Color::from_rgba8(255, 0, 0, 255),
                Color::from_rgba8(0, 255, 0, 255),
            );
        let top = canvas.pixel(14, 8).unwrap();
        let bottom = canvas.pixel(14, 24).unwrap();
        assert!(top.red() > 200 && top.green() < 100);
        assert!(bottom.green() > 200 && bottom.red() < 100);
        assert_eq!(canvas.pixel(3, 15).unwrap().alpha(), 0);
        assert_eq!(canvas.pixel(14, 27).unwrap().alpha(), 0);
    }
}
