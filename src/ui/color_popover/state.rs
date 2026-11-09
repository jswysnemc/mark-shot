mod conversion;
use super::layout;
use crate::ui::radial_palette::RadialPalette;
use conversion::color_bytes;
pub use conversion::{
    color_channel_u8, color_to_hex_string, color_to_hsv, color_with_channel, hsv_to_color,
    parse_hex_color, step_hex_text,
};

/// 【属性取色】【命中判断】返回 point 是否位于 rect 内，排除右侧和下侧的控件外坐标
fn point_in_rect(point: (f64, f64), rect: Rect) -> bool {
    point.0 >= rect.left() as f64
        && point.0 < rect.right() as f64
        && point.1 >= rect.top() as f64
        && point.1 < rect.bottom() as f64
}

use crate::config::ColorPicker;
use crate::interaction::ScrollAccumulator;
use crate::theme::anim;
use crate::types::tool_settings::default_color;
use crate::ui::panel::{AnimatedPanel, HoverablePanel, PanelItem, UiPanel};
use crate::ui::text_field::TextFieldGroup;
use std::time::{Duration, Instant};
use tiny_skia::{Color, Mask, Pixmap, Rect};

fn cfg() -> &'static ColorPicker {
    &crate::config::get().color_picker
}

/// 【属性取色】【字体大小】返回原版唯一十六进制字段的十二像素字号
pub fn field_font_size() -> f32 {
    12.0
}
/// 【属性取色】【面板宽度】返回 Qt 实际渲染的固定逻辑宽度，不读取旧布局选项
pub fn width() -> f32 {
    layout::WIDTH
}
/// 【属性取色】【面板高度】返回 Qt 实际渲染的固定逻辑高度
pub fn height() -> f32 {
    layout::HEIGHT
}
/// 【属性取色】【色块大小】返回方块预览半边长，保留旧调用者的颜色几何接口
pub fn swatch_radius() -> f32 {
    layout::SWATCH_SIZE / 2.0
}

#[derive(Debug, Clone, Copy)]
pub enum ColorPickerItem {}

impl PanelItem for ColorPickerItem {
    fn size(&self) -> f32 {
        match *self {}
    }
    fn trailing_padding(&self) -> f32 {
        match *self {}
    }
    fn is_button(&self) -> bool {
        match *self {}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorField {
    Hex,
    R,
    G,
    B,
    A,
}

pub const RGBA_FIELDS: [ColorField; 4] =
    [ColorField::R, ColorField::G, ColorField::B, ColorField::A];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorTarget {
    Main,
    Outline,
    TextBackground,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPopoverElement {
    SvSquare,
    HueSlider,
    AlphaSlider,
    Swatch(usize),
    Eyedropper,
    Field(ColorField),
}

/// 【属性取色】【历史色块】从 current 与 history 生成当前色和最多七项历史，不填入 LumineCapture 默认调色板
fn palette_row(current: Color, history: &[Color]) -> Vec<Color> {
    let mut row = vec![current];
    for color in history.iter().copied() {
        if !row
            .iter()
            .any(|item| color_bytes(*item) == color_bytes(color))
        {
            row.push(color);
        }
        if row.len() == 8 {
            break;
        }
    }
    row
}

// ─── SV-square state (Content of the pixmap is built by the render) ──────────────
pub struct ColorSquareState {
    pub exact_color: Option<Color>,
    pub hue: f32,
    pub sv: (f32, f32),
    pub alpha: u8,
    pub sv_pixmap: Option<Pixmap>,
    pub sv_dirty: bool,
    pub dragging: bool,
}

impl Default for ColorSquareState {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorSquareState {
    pub fn new() -> Self {
        let (h, s, v) = color_to_hsv(default_color().color());
        let alpha = default_color().alpha();
        Self {
            exact_color: Some(default_color().color()),
            hue: h,
            sv: (s, v),
            alpha,
            sv_pixmap: None,
            sv_dirty: true,
            dragging: false,
        }
    }

    pub fn set_hue(&mut self, hue: f32) {
        let hue = hue.rem_euclid(360.0);
        if (self.hue - hue).abs() > f32::EPSILON {
            self.exact_color = None;
            self.hue = hue;
            self.sv_dirty = true;
        }
    }

    pub fn color(&self) -> Color {
        if let Some(color) = self.exact_color {
            return color;
        }
        let rgb = hsv_to_color(self.hue, self.sv.0, self.sv.1).to_color_u8();
        Color::from_rgba8(rgb.red(), rgb.green(), rgb.blue(), self.alpha)
    }
}

// ── hue slider geometry ─────────────────────────────────────────

pub struct ColorPickerPopover {
    pub colorpicker_pixmap: Option<Pixmap>,

    pub position: (f32, f32),
    pub size: (f32, f32),
    pub opacity: f32,
    pub monitor_idx: usize,

    pub open: bool,
    pub dirty: bool,

    pub hovered: Option<ColorPopoverElement>,

    pub last_tick: Option<Instant>,
    pub render_pos: (f32, f32),

    pub sv_square: ColorSquareState,

    pub hue_track_pixmap: Option<Pixmap>, // content is built by render, built once
    pub hue_dragging: bool,
    pub alpha_dragging: bool,
    pub radial: Option<RadialPalette>,
    pub picker_fonts: Option<super::fonts::PickerFonts>,

    pub sv_clip_mask: Option<Mask>,
    pub hue_clip_mask: Option<Mask>,

    pub picking: bool,
    pub target: ColorTarget,
    pub recent_colors: Vec<Color>,
    pub outline_palette: Vec<Color>,
    pub history: Vec<Color>,
    pub initial_history: Vec<Color>,
    pub fields: TextFieldGroup<ColorField>,
    pub pre_edit_snapshot: Option<crate::editor::history_snapshot::HistorySnapshot>,
    pub(crate) pre_edit_checkpointed: bool,

    /// Scroll accumulator for scrollable fields (hex/rgba)
    /// Keeps scroll fractional state separate from raw events to avoid
    /// processing every single scroll event, trackpad or mouse wheel held
    /// will flood thousands of events that would freeze
    pub scroll: ScrollAccumulator<ColorField>,
}

impl Default for ColorPickerPopover {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorPickerPopover {
    pub fn new() -> Self {
        let history: Vec<Color> = crate::editor::saved::get()
            .recent_colors
            .iter()
            .map(|c| c.color())
            .take(7)
            .collect();
        Self {
            colorpicker_pixmap: None,
            position: (0.0, 0.0),
            size: (width(), height()),
            opacity: 0.0,
            monitor_idx: 0,
            open: false,
            dirty: false,
            hovered: None,
            last_tick: None,
            render_pos: (0.0, 0.0),
            sv_square: ColorSquareState::new(),
            hue_track_pixmap: None,
            hue_dragging: false,
            alpha_dragging: false,
            radial: None,
            picker_fonts: None,
            sv_clip_mask: None,
            hue_clip_mask: None,
            picking: false,
            target: ColorTarget::Main,
            recent_colors: palette_row(default_color().color(), &history),
            outline_palette: cfg()
                .outline_palette
                .iter()
                .map(|c| c.color())
                .take(7)
                .collect(),
            initial_history: history.clone(),
            history,
            fields: TextFieldGroup::new(),
            scroll: ScrollAccumulator::new(),
            pre_edit_snapshot: None,
            pre_edit_checkpointed: false,
        }
    }

    /// 【属性取色】【事务取出】返回尚未提交的颜色快照并清除状态，已经提前记录历史的群组事务返回空值，无参数
    pub(crate) fn take_edit_snapshot(
        &mut self,
    ) -> Option<crate::editor::history_snapshot::HistorySnapshot> {
        let snapshot = self.pre_edit_snapshot.take();
        if std::mem::take(&mut self.pre_edit_checkpointed) {
            None
        } else {
            snapshot
        }
    }

    pub fn hit_test(&self, local: (f64, f64)) -> bool {
        let Some(rect) = self.rect() else {
            return false;
        };
        point_in_rect(local, rect)
    }

    /// 【属性取色】【共享命中】按local返回实际取色子控件，原生窗口与画布使用同一几何
    pub fn element_at(&self, local: (f64, f64)) -> Option<ColorPopoverElement> {
        if self.sv_square_hit(local) {
            Some(ColorPopoverElement::SvSquare)
        } else if self.hue_slider_hit(local) {
            Some(ColorPopoverElement::HueSlider)
        } else if self.alpha_slider_hit(local) {
            Some(ColorPopoverElement::AlphaSlider)
        } else if self.hex_field_hit(local) {
            Some(ColorPopoverElement::Field(ColorField::Hex))
        } else {
            self.swatch_hit(local).map(ColorPopoverElement::Swatch)
        }
    }

    /// 【属性取色】【明度范围】返回当前面板的 SV 控件矩形，环形快捷色板没有该控件
    pub fn sv_square_rect(&self) -> Option<Rect> {
        if self.radial.is_some() {
            return None;
        }
        let rect = self.rect()?;
        Some(layout::sv((rect.left(), rect.top())))
    }
    /// 【属性取色】【明度命中】判断 local 是否位于 SV 控件，不穿透到截图画布
    pub fn sv_square_hit(&self, local: (f64, f64)) -> bool {
        self.sv_square_rect()
            .is_some_and(|rect| point_in_rect(local, rect))
    }
    /// 【属性取色】【明度输入】按 local 整数像素映射到 0–255 饱和度和明度，返回状态保存在 sv_square
    pub fn set_sv_from_local(&mut self, local: (f64, f64)) {
        let Some(rect) = self.sv_square_rect() else {
            return;
        };
        let saturation = layout::value(rect, local, 255);
        let value = (255.0
            - (local.1.round() - rect.top() as f64) / (rect.height() - 1.0) as f64 * 255.0)
            .round()
            .clamp(0.0, 255.0);
        let sv = (saturation as f32 / 255.0, value as f32 / 255.0);
        if self.sv_square.sv != sv {
            self.sv_square.sv = sv;
            self.sv_square.exact_color = None;
        }
    }
    /// 【属性取色】【色相范围】返回当前面板的横向色相矩形，快捷色板没有轨道
    pub fn hue_slider_rect(&self) -> Option<Rect> {
        if self.radial.is_some() {
            return None;
        }
        let rect = self.rect()?;
        Some(layout::hue((rect.left(), rect.top())))
    }
    /// 【属性取色】【色相命中】判断 local 是否位于横向色相轨道，不添加旧垂直控件留量
    pub fn hue_slider_hit(&self, local: (f64, f64)) -> bool {
        self.hue_slider_rect()
            .is_some_and(|rect| point_in_rect(local, rect))
    }
    /// 【属性取色】【色相输入】将 local 横坐标映射为 0–359 度，刷新明度图像
    pub fn set_hue_from_local(&mut self, local: (f64, f64)) {
        if let Some(rect) = self.hue_slider_rect() {
            self.sv_square
                .set_hue(layout::value(rect, local, 359) as f32);
        }
    }
    /// 【属性取色】【透明范围】返回 alpha 轨道矩形，快捷色板没有透明度控件
    pub fn alpha_slider_rect(&self) -> Option<Rect> {
        if self.radial.is_some() {
            return None;
        }
        let rect = self.rect()?;
        Some(layout::alpha((rect.left(), rect.top())))
    }
    /// 【属性取色】【透明命中】判断 local 是否位于横向透明度轨道
    pub fn alpha_slider_hit(&self, local: (f64, f64)) -> bool {
        self.alpha_slider_rect()
            .is_some_and(|rect| point_in_rect(local, rect))
    }
    /// 【属性取色】【透明输入】按 local 横坐标设置 0–255 alpha，不改变 HSV 值
    pub fn set_alpha_from_local(&mut self, local: (f64, f64)) {
        if let Some(rect) = self.alpha_slider_rect() {
            let alpha = layout::value(rect, local, 255) as u8;
            if self.sv_square.alpha != alpha {
                self.sv_square.alpha = alpha;
                self.sv_square.exact_color = None;
            }
        }
    }

    pub fn palette(&self) -> &[Color] {
        &self.recent_colors
    }

    /// 【属性取色】【历史记录】把 color 写入七项去重历史，不在 HSV 手势期间重排正在显示的历史色块
    pub fn record_used_color(&mut self, color: Color) {
        let bytes = color_bytes(color);
        self.history.retain(|item| color_bytes(*item) != bytes);
        self.history.insert(0, color);
        self.history.truncate(7);
    }
    /// 【属性取色】【历史刷新】在打开面板或明确选择颜色后刷新当前色与历史色块
    pub fn refresh_palette(&mut self) {
        self.recent_colors = palette_row(self.sv_square.color(), &self.history);
    }

    pub fn select_color(&mut self, color: Color) {
        let (h, s, v) = color_to_hsv(color);
        // 1. 【属性取色】【灰度输入】无色相输入保留上次色相，与 Qt 的灰度处理一致
        if s > f32::EPSILON {
            self.sv_square.set_hue(h.floor().min(359.0));
        }
        self.sv_square.sv = ((s * 255.0).round() / 255.0, (v * 255.0).round() / 255.0);
        self.sv_square.alpha = color.to_color_u8().alpha();
        self.sv_square.exact_color = Some(color);
    }

    /// 【属性取色】【色块命中】按 local 返回可用历史索引；当前色不可点击，重叠部分以最上层色块优先
    pub fn swatch_hit(&self, local: (f64, f64)) -> Option<usize> {
        if self.radial.is_some() {
            return None;
        }
        let rect = self.rect()?;
        let origin = (rect.left(), rect.top());
        if !point_in_rect(local, layout::content(origin)) {
            return None;
        }
        (1..self.recent_colors.len())
            .rev()
            .find(|index| point_in_rect(local, layout::history(origin, *index).unwrap()))
    }
    /// 【属性取色】【旧取色按钮】原版此面板没有吸管按钮，local 不触发额外控件
    pub fn eyedropper_hit(&self, _local: (f64, f64)) -> bool {
        false
    }
    /// 【属性取色】【字段范围】返回唯一十六进制字段矩形，快捷色板没有文本输入
    pub fn hex_field_rect(&self) -> Option<Rect> {
        if self.radial.is_some() {
            return None;
        }
        let rect = self.rect()?;
        Some(layout::hex((rect.left(), rect.top())))
    }
    /// 【属性取色】【字段命中】判断 local 是否位于十六进制输入框
    pub fn hex_field_hit(&self, local: (f64, f64)) -> bool {
        self.hex_field_rect()
            .is_some_and(|rect| point_in_rect(local, rect))
    }
    /// 【属性取色】【旧通道布局】原版没有独立 RGBA 字段，field 不对应可见控件
    pub fn rgba_field_rect(&self, _field: ColorField) -> Option<Rect> {
        None
    }
    /// 【属性取色】【旧通道命中】local 不触发已移除的 RGBA 数值行
    pub fn rgba_field_hit(&self, _local: (f64, f64)) -> Option<ColorField> {
        None
    }

    pub fn field_rect(&self, field: ColorField) -> Option<Rect> {
        match field {
            ColorField::Hex => self.hex_field_rect(),
            _ => self.rgba_field_rect(field),
        }
    }

    pub fn field_text(&self, field: ColorField) -> String {
        if let Some(edit) = self.fields.editing.as_ref()
            && edit.key == field
        {
            return edit.field.text.clone();
        }
        self.fields.value(field).cloned().unwrap_or_default()
    }

    pub fn confirmed_field_text(&self, field: ColorField) -> String {
        let color = self.sv_square.color();
        match field {
            ColorField::Hex => format!("#{}", color_to_hex_string(color)),
            _ => color_channel_u8(color, field).to_string(),
        }
    }

    pub fn sync_field_values(&mut self) {
        let color = self.sv_square.color();
        if let Some(current) = self.recent_colors.first_mut() {
            *current = color;
        }
        self.fields
            .sync_value(ColorField::Hex, format!("#{}", color_to_hex_string(color)));
        for field in RGBA_FIELDS {
            self.fields
                .sync_value(field, color_channel_u8(color, field).to_string());
        }
    }

    pub fn try_apply_hex_text(&mut self, text: &str) -> Option<Color> {
        let color = parse_hex_color(text)?;
        self.select_color(color);
        Some(color)
    }

    pub fn try_apply_rgba_text(&mut self, field: ColorField, text: &str) -> Option<Color> {
        let value: u8 = text.trim().parse().ok()?;
        let current = self.sv_square.color();
        let color = color_with_channel(current, field, value);
        self.select_color(color);
        Some(color)
    }

    /// Accumulate a fractional scroll `delta` (already in "steps", not raw
    /// pixels) for the given field and return how many whole steps have
    /// accumulated since the last call (may be negative). Switching fields
    /// discards the leftover from the previous one.
    ///
    /// Delegates to [`ScrollAccumulator`], see its docs for the two
    /// flood-prevention safeguards (rate limit + step cap).
    pub fn scroll_step(&mut self, field: ColorField, delta: f32) -> i32 {
        self.scroll.step(field, delta)
    }

    /// scroll reset. Call on any non-scroll user action
    /// (click, key press) so stale queued events don't trigger afterwards
    pub fn cancel_scroll(&mut self) {
        self.scroll.cancel();
    }
}

impl UiPanel for ColorPickerPopover {
    type Item = ColorPickerItem;

    fn render_pos(&self) -> (f32, f32) {
        self.render_pos
    }
    fn size(&self) -> (f32, f32) {
        self.size
    }
    fn items(&self) -> &[Self::Item] {
        &[]
    }
    fn padding(&self) -> f32 {
        cfg().padding
    }
    fn monitor_idx(&self) -> usize {
        self.monitor_idx
    }
    fn set_dirty(&mut self) {
        self.dirty = true;
    }
    fn is_dirty(&self) -> bool {
        self.dirty
    }
    fn is_visible(&self) -> bool {
        self.open || self.opacity > 0.0
    }

    fn rect(&self) -> Option<Rect> {
        if self.opacity <= 0.0 {
            return None;
        }
        let (x, y) = self.render_pos();
        let (w, h) = self.size();
        Rect::from_xywh(x, y, w, h)
    }
}

impl HoverablePanel for ColorPickerPopover {
    type Hover = Option<ColorPopoverElement>;
    fn hovered(&self) -> Self::Hover {
        self.hovered
    }
    fn set_hovered(&mut self, hover: Self::Hover) {
        self.hovered = hover;
    }
}

impl AnimatedPanel for ColorPickerPopover {
    fn last_tick(&self) -> Option<Instant> {
        self.last_tick
    }
    fn set_last_tick(&mut self, at: Option<Instant>) {
        self.last_tick = at;
    }

    fn anim_interval(&self) -> Duration {
        anim::frame()
    }
    fn anim_dt(&self) -> f32 {
        anim::dt()
    }

    fn is_animating(&self) -> bool {
        let target = if self.open { 1.0 } else { 0.0 };
        (self.opacity - target).abs() > anim::OPACITY_EPSILON
    }

    fn animate_step(&mut self, dt: f32) -> bool {
        let target = if self.open { 1.0 } else { 0.0 };
        if (self.opacity - target).abs() > anim::OPACITY_EPSILON {
            let delta = anim::popover_fade() * crate::config::get().animation.speed * dt;
            self.opacity += (target - self.opacity).signum() * delta;
            self.opacity = self.opacity.clamp(0.0, 1.0);
            true
        } else {
            false
        }
    }
}
