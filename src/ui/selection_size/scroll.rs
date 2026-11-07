//! 【选区尺寸】【字段视口】保留单行输入横向偏移，仅在光标越过内部边缘时滚动

use std::cell::Cell;

#[derive(Default)]
pub(crate) struct Scroll {
    position: Cell<Option<i32>>,
}

impl Scroll {
    /// 【选区尺寸】【文字偏移】按 width、caret 与 field_width 返回字段内文字起点，短文字居中，长文字保留原偏移
    pub fn x(&self, width: f32, caret: f32, field_width: f32) -> f32 {
        // 1. 【选区尺寸】【内部留白】原版边框、样式内边距和单行输入留白共五个逻辑像素
        let padding = 5.0;
        let available = (field_width - padding * 2.0).round().max(1.0) as i32;
        let text_width = width.round() as i32 + 1;
        let cursor = caret.round() as i32;
        if text_width <= available {
            let scroll = (text_width - available) / 2;
            self.position.set(Some(scroll));
            return padding - scroll as f32;
        }
        // 2. 【选区尺寸】【可见光标】仅在光标离开当前视口时平移，双击不能因第一次点击而无故重置滚动
        let mut scroll = self.position.get().unwrap_or(0);
        if cursor - scroll >= available {
            scroll = cursor - available + 1;
        } else if cursor - scroll < 0 && scroll < text_width {
            scroll = cursor;
        } else if text_width - scroll < available {
            // 3. 【选区尺寸】【末端回收】自然宽度取整后额外保留一列，旧偏移越过新正文末端时回收空白
            scroll = text_width - available + 1;
        } else {
            scroll = scroll.max(0);
        }
        self.position.set(Some(scroll));
        padding - scroll as f32
    }
}
