//! 【X11输入法】【候选编辑】XIM字符位置转换为共享UTF-8字节位置，支持局部候选替换

#[derive(Default)]
pub(super) struct Composition {
    text: String,
    caret: usize,
}

impl Composition {
    /// 【X11输入法】【候选清除】无参数，清空尚未确认文字与光标位置
    pub fn clear(&mut self) {
        self.text.clear();
        self.caret = 0;
    }

    /// 【X11输入法】【局部替换】按first、length、text和caret更新字符范围，返回范围是否合法
    pub fn replace(&mut self, first: i32, length: i32, text: &str, caret: i32) -> bool {
        let (Ok(first), Ok(length), Ok(caret)) = (
            usize::try_from(first),
            usize::try_from(length),
            usize::try_from(caret),
        ) else {
            return false;
        };
        let boundaries = self
            .text
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(self.text.len()))
            .collect::<Vec<_>>();
        let Some(end) = first.checked_add(length) else {
            return false;
        };
        let (Some(start), Some(end)) = (boundaries.get(first), boundaries.get(end)) else {
            return false;
        };
        if self.text.len() - (end - start) + text.len() > 65536 {
            return false;
        }
        self.text.replace_range(*start..*end, text);
        self.caret = caret.min(self.text.chars().count());
        true
    }

    /// 【X11输入法】【光标移动】按position更新候选字符位置，返回钳制后的实际位置
    pub fn move_caret(&mut self, position: i32) -> i32 {
        self.caret = usize::try_from(position)
            .unwrap_or_default()
            .min(self.text.chars().count());
        self.caret as i32
    }

    /// 【X11输入法】【候选快照】无参数，返回带UTF-8字节光标的共享候选
    pub fn snapshot(&self) -> crate::types::text_input::Preedit {
        let cursor = self
            .text
            .char_indices()
            .nth(self.caret)
            .map_or(self.text.len(), |(index, _)| index);
        crate::types::text_input::Preedit {
            text: self.text.clone(),
            cursor: Some((cursor, cursor)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【X11输入法】【字符坐标回归】验证非ASCII候选局部替换使用字符范围，非法范围不修改文字，无参数
    #[test]
    fn partial_updates_use_character_offsets_and_utf8_cursors() {
        let mut composition = Composition::default();
        assert!(composition.replace(0, 0, "中文ni", 4));
        assert_eq!(composition.snapshot().cursor, Some((8, 8)));
        assert!(composition.replace(2, 2, "你好", 3));
        assert_eq!(composition.snapshot().text, "中文你好");
        assert_eq!(composition.snapshot().cursor, Some((9, 9)));
        assert!(!composition.replace(10, 1, "x", 1));
        assert_eq!(composition.snapshot().text, "中文你好");
        assert!(composition.replace(0, 4, "", 0));
        assert_eq!(
            composition.snapshot(),
            crate::types::text_input::Preedit::default()
        );
    }
}
