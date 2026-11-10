//! 【文字属性栏】【窄屏换行】字号滑条、透明度滑条和按钮按组换行，所有命中区域随绘制同步

use super::layout::TextLayout;
use tiny_skia::Rect;

impl TextLayout {
    /// 【文字属性栏】【宽度约束】按 available 返回屏幕内的分组布局，空间充足时保持原有几何
    pub fn fit(mut self, available: f32) -> Self {
        if self.size.0 <= available {
            return self;
        }
        let margin = 8.0;
        let row = self.size.1;
        let mut x = margin;
        let mut y = 0.0;
        let mut right: f32 = margin;
        for index in 0..self.controls.len() {
            let Some(control) = self.controls[index] else {
                continue;
            };
            let left = if index < 2 {
                self.glyphs[index].left()
            } else {
                control.left()
            };
            let width = if index < 2 {
                self.values[index].right() - left
            } else {
                control.width()
            };
            if x > margin && x + width > available - margin {
                x = margin;
                y += row;
            }
            let shifted = |rect: Rect| {
                Rect::from_xywh(
                    rect.x() - left + x,
                    rect.y() + y,
                    rect.width(),
                    rect.height(),
                )
                .unwrap()
            };
            self.controls[index] = Some(shifted(control));
            if index < 2 {
                self.glyphs[index] = shifted(self.glyphs[index]);
                self.values[index] = shifted(self.values[index]);
            }
            right = right.max(x + width);
            x += width + 8.0;
        }
        self.size = (right + margin, y + row);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::text_properties::layout::Control;

    /// 【文字属性测试】【放大换行】无参数，最大字号在窄窗口内保留全部控件且绘制区域与命中一致
    #[test]
    fn large_font_wraps_groups_without_clipping_or_changing_hit_targets() {
        let layout = TextLayout::new(56.0, 24.0, 48.0, true).fit(624.0);
        assert!(layout.size.0 <= 624.0);
        assert!(layout.size.1 > 80.0);
        for (index, cell) in layout.controls.iter().enumerate() {
            let cell = cell.unwrap();
            assert!(cell.left() >= 0.0 && cell.right() <= layout.size.0);
            assert!(cell.top() >= 0.0 && cell.bottom() <= layout.size.1);
            assert_eq!(
                layout.hit((
                    cell.x() + cell.width() / 2.0,
                    cell.y() + cell.height() / 2.0
                )),
                Control::from_index(index)
            );
        }
    }
}
