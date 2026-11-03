//! 【输入法】【有限上下文】只复制协议允许的四千字节，完整保留光标与选区，不切开 UTF-8 码点

use crate::types::text_input::Surrounding;

pub const MAX_BYTES: usize = 4000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window<'a> {
    pub text: &'a str,
    pub cursor: i32,
    pub anchor: i32,
    pub document_start: usize,
}

impl Window<'_> {
    /// 【输入法】【上下文比较】按 value 比较借用窗口与已有快照，返回是否完全相同；不分配字符串
    pub fn matches(&self, value: &Surrounding) -> bool {
        self.text == value.text
            && self.cursor == value.cursor
            && self.anchor == value.anchor
            && self.document_start == value.document_start
    }

    /// 【输入法】【上下文保存】复制当前有限窗口并返回协议快照，无参数；只在实际上下文变化时调用
    pub fn owned(self) -> Surrounding {
        Surrounding {
            text: self.text.to_owned(),
            cursor: self.cursor,
            anchor: self.anchor,
            document_start: self.document_start,
        }
    }
}

/// 【输入法】【上下文裁切】按 text、cursor、anchor 文档字节返回协议上下文；位置非法、上下文含空字符或完整选区超过上限时返回空值
pub fn window(text: &str, cursor: usize, anchor: usize) -> Option<Surrounding> {
    view(text, cursor, anchor).map(Window::owned)
}

/// 【输入法】【上下文借用】按 text、cursor、anchor 返回合法有限窗口，非法上下文返回空值；不复制文档文字
pub fn view(text: &str, cursor: usize, anchor: usize) -> Option<Window<'_>> {
    if !text.is_char_boundary(cursor) || !text.is_char_boundary(anchor) {
        return None;
    }
    let low = cursor.min(anchor);
    let high = cursor.max(anchor);
    if high - low > MAX_BYTES {
        return None;
    }
    // 1. 【输入法】【选区优先】先保留完整选区，再分配前后上下文，接近文档边缘时复用剩余空间
    let allowance = MAX_BYTES - (high - low);
    let mut start = low.saturating_sub(allowance / 2);
    let mut end = text.len().min(start + MAX_BYTES);
    start = start.min(end.saturating_sub(MAX_BYTES));
    if end < high {
        end = high;
        start = end.saturating_sub(MAX_BYTES);
    }
    // 2. 【输入法】【码点边界】只向窗口内部移动裁切边，光标和选区本身已在合法 UTF-8 边界上
    while !text.is_char_boundary(start) {
        start += 1;
    }
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let text = text.get(start..end)?;
    if text.contains('\0') {
        return None;
    }
    Some(Window {
        text,
        cursor: i32::try_from(cursor - start).ok()?,
        anchor: i32::try_from(anchor - start).ok()?,
        document_start: start,
    })
}
