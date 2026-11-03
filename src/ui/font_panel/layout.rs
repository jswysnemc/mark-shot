//! 【字体弹层】【分区几何】搜索框、可见字体列表和底部字号控件保留实际 Qt 尺寸与间距

use tiny_skia::Rect;

pub const ROW_HEIGHT: f32 = 28.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontLayout {
    pub size: (f32, f32),
    pub search: Rect,
    pub list: Rect,
    pub size_field: Rect,
    pub bold: Rect,
    pub italic: Rect,
}

impl FontLayout {
    /// 【字体弹层】【分区计算】按 total_rows 和 available 返回原版三分区；查询过滤不改变 total_rows，列表最多显示十行
    pub fn new(total_rows: usize, available: (f32, f32)) -> Self {
        let width = 360.0_f32.min((available.0.floor() - 16.0).max(180.0));
        let chrome = 80.0;
        let preferred = chrome + total_rows.clamp(1, 10) as f32 * ROW_HEIGHT;
        let height = preferred.min((available.1.floor() - 16.0).max(chrome + ROW_HEIGHT * 3.0));
        let inner = width - 12.0;
        let controls_y = height - 34.0;
        let rect = |x, y, w, h| Rect::from_xywh(x, y, w, h).unwrap();
        Self {
            size: (width, height),
            search: rect(6.0, 7.0, inner, 28.0),
            list: rect(6.0, 40.0, inner, (controls_y - 46.0).max(ROW_HEIGHT)),
            size_field: rect(6.0, controls_y, inner - 60.0, 26.0),
            bold: rect(width - 62.0, controls_y, 26.0, 26.0),
            italic: rect(width - 32.0, controls_y, 26.0, 26.0),
        }
    }

    /// 【字体弹层】【锚点定位】根据 properties、font_button 和 bounds 返回下方七像素间距位置；空间不足时翻转并限制留白
    pub fn position(self, properties: Rect, font_button: Rect, bounds: (f32, f32)) -> (f32, f32) {
        let mut y = properties.bottom() + 7.0;
        if y + self.size.1 > bounds.1 - 8.0 {
            y = properties.top() - 7.0 - self.size.1;
        }
        (
            (font_button.left() + font_button.width() / 2.0 - self.size.0 / 2.0)
                .floor()
                .clamp(8.0, (bounds.0 - self.size.0 - 8.0).max(8.0)),
            y.clamp(8.0, (bounds.1 - self.size.1 - 8.0).max(8.0)),
        )
    }

    /// 【字体弹层】【行位置】返回 row 在 offset 滚动像素下的列表本地矩形；绘制方按列表区域裁切部分行
    pub fn row(self, row: usize, offset: f32, scrollbar: bool) -> Rect {
        Rect::from_xywh(
            self.list.left(),
            self.list.top() + row as f32 * ROW_HEIGHT - offset,
            self.list.width() - if scrollbar { 14.0 } else { 0.0 },
            ROW_HEIGHT,
        )
        .unwrap()
    }
}
