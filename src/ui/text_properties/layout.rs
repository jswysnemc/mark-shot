//! 【文字属性栏】【实际几何】尺寸来自原版 ShotWindow 控件树，不复用旧属性栏的步进输入布局

use tiny_skia::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Size,
    Opacity,
    Color,
    Background,
    Font,
    Edit,
}

impl Control {
    /// 【文字属性栏】【控件索引】返回统一属性控件索引，供现有颜色按钮和损坏队列使用
    pub fn index(self) -> usize {
        self as usize
    }

    /// 【文字属性栏】【索引转换】按 index 返回控件类型，越界返回空值
    pub fn from_index(index: usize) -> Option<Self> {
        [
            Self::Size,
            Self::Opacity,
            Self::Color,
            Self::Background,
            Self::Font,
            Self::Edit,
        ]
        .get(index)
        .copied()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextLayout {
    pub size: (f32, f32),
    pub glyphs: [Rect; 2],
    pub values: [Rect; 2],
    pub controls: [Option<Rect>; 6],
    pub value_font_size: f32,
    pub icon_size: f32,
}

impl TextLayout {
    /// 【文字属性栏】【滑条手柄】按 cell、value 与 range 返回实际 Qt 样式手柄，整数位置采用原版四舍五入
    pub fn slider_handle(cell: Rect, value: i32, range: (i32, i32)) -> Rect {
        let ratio = ((value - range.0) as f32 / (range.1 - range.0).max(1) as f32).clamp(0.0, 1.0);
        Rect::from_xywh(
            cell.left() + ((cell.width() - 14.0) * ratio).round(),
            cell.top() + ((cell.height() - 12.0) / 2.0).floor(),
            14.0,
            12.0,
        )
        .unwrap()
    }

    /// 【文字属性栏】【滑条数值】将手柄左边相对 cell 的 x 坐标换算到 range，返回边界内整数值
    pub fn slider_value(cell: Rect, x: f32, range: (i32, i32)) -> i32 {
        let ratio = ((x - cell.left()) / (cell.width() - 14.0).max(1.0)).clamp(0.0, 1.0);
        range.0 + ((range.1 - range.0) as f32 * ratio).round() as i32
    }
    /// 【文字属性栏】【布局构造】按 button_size、font_size、icon_size 和 editing_available 返回原版缩放布局，不调整字号内容
    pub fn new(button_size: f32, font_size: f32, icon_size: f32, editing_available: bool) -> Self {
        let button_size = button_size.round().clamp(24.0, 64.0);
        let font_size = font_size.round().clamp(8.0, 24.0);
        let scale = (font_size / 11.0).max(1.0);
        let margin_x = (8.0 * scale).round().max(8.0);
        let margin_y = (6.0 * scale).round().max(6.0);
        let spacing = (6.0 * scale).round().max(6.0);
        let minor = (2.0 * scale).round().max(2.0);
        let glyph_width = (font_size + 8.0).min(button_size - 8.0).max(18.0);
        let glyph_height = (button_size - 2.0).max(28.0);
        // 1. 【文字属性栏】【按钮边框】原版没有给属性按钮设置固定尺寸，样式表的边框在三十像素内容外再占两像素
        let button = button_size + 2.0;
        let content_height = button.max(glyph_height);
        let height = content_height + margin_y * 2.0;
        let width = |base: f32| (base * scale).round().max(base);
        let rect = |x, y, w, h| Rect::from_xywh(x, y, w, h).unwrap();
        let mut x = margin_x + minor;
        let first_glyph = rect(
            x,
            ((height - glyph_height) / 2.0).floor(),
            glyph_width,
            glyph_height,
        );
        x += glyph_width + spacing;
        let size_slider = rect(x, ((height - 15.0) / 2.0).floor(), width(88.0), 15.0);
        x += width(88.0) + spacing;
        let size_value = rect(x, margin_y, width(34.0), content_height);
        x += width(34.0) + spacing + minor;
        let opacity_glyph = rect(
            x,
            ((height - glyph_height) / 2.0).floor(),
            glyph_width,
            glyph_height,
        );
        x += glyph_width + spacing;
        let opacity_slider = rect(x, ((height - 15.0) / 2.0).floor(), width(76.0), 15.0);
        x += width(76.0) + spacing;
        let opacity_value = rect(x, margin_y, width(36.0), content_height);
        x += width(36.0) + spacing + minor;
        let mut controls = [
            Some(size_slider),
            Some(opacity_slider),
            None,
            None,
            None,
            None,
        ];
        for (index, slot) in controls.iter_mut().enumerate().skip(2) {
            if index == Control::Edit.index() && !editing_available {
                break;
            }
            *slot = Some(rect(x, margin_y, button, button));
            x += button + spacing;
        }
        Self {
            size: (x - spacing + margin_x, height),
            glyphs: [first_glyph, opacity_glyph],
            values: [size_value, opacity_value],
            controls,
            value_font_size: (font_size - 1.0).max(8.0),
            icon_size: icon_size.round().min(button_size - 8.0).max(20.0),
        }
    }

    /// 【文字属性栏】【控件命中】将面板本地 point 映射到控件，图形标签、数值标签和留白均不接受按钮输入
    pub fn hit(self, point: (f32, f32)) -> Option<Control> {
        self.controls.iter().enumerate().find_map(|(index, rect)| {
            rect.filter(|rect| {
                point.0 >= rect.left()
                    && point.0 < rect.right()
                    && point.1 >= rect.top()
                    && point.1 < rect.bottom()
            })
            .and_then(|_| Control::from_index(index))
        })
    }

    /// 【文字属性栏】【控件平移】返回 control 在 origin 对应面板中的矩形，不可用编辑按钮返回空值
    pub fn control(self, control: Control, origin: (f32, f32)) -> Option<Rect> {
        self.controls[control.index()].and_then(|rect| {
            Rect::from_xywh(
                origin.0 + rect.left(),
                origin.1 + rect.top(),
                rect.width(),
                rect.height(),
            )
        })
    }
}
