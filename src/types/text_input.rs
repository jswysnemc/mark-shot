//! 【输入法】【消息契约】候选与确认分别传递，位置使用 UTF-8 字节，不进入持久标注

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preedit {
    pub text: String,
    pub cursor: Option<(usize, usize)>,
}

impl Default for Preedit {
    /// 【输入法】【初始候选】返回协议空字符串与零字节光标位置，不等同于显式隐藏光标，无参数
    fn default() -> Self {
        Self {
            text: String::new(),
            cursor: Some((0, 0)),
        }
    }
}

impl Preedit {
    /// 【输入法】【候选校验】按 text、begin、end 返回候选；双 -1 隐藏光标，非法 UTF-8 字节位置返回空值
    pub fn from_protocol(text: String, begin: i32, end: i32) -> Option<Self> {
        if begin == -1 && end == -1 {
            return Some(Self { text, cursor: None });
        }
        let begin = usize::try_from(begin).ok()?;
        let end = usize::try_from(end).ok()?;
        if !text.is_char_boundary(begin) || !text.is_char_boundary(end) {
            return None;
        }
        Some(Self {
            text,
            cursor: Some((begin, end)),
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Batch {
    pub preedit: Preedit,
    pub commit: String,
    pub delete_before: u32,
    pub delete_after: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Replacement {
    pub start: isize,
    pub length: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Edit {
    pub preedit: Preedit,
    pub commit: String,
    pub replacement: Option<Replacement>,
}

impl Edit {
    /// 【输入法】【编辑转换】将 batch 的前后字节删除转换为相对替换，返回编辑事件；长度溢出时返回空值，不复制确认文字
    pub fn from_batch(batch: Batch) -> Option<Self> {
        let before = usize::try_from(batch.delete_before).ok()?;
        let after = usize::try_from(batch.delete_after).ok()?;
        let length = before.checked_add(after)?;
        let start = -isize::try_from(before).ok()?;
        Some(Self {
            preedit: batch.preedit,
            commit: batch.commit,
            replacement: (length != 0).then_some(Replacement { start, length }),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Update {
    pub target_id: u64,
    pub serial: u32,
    pub sync_allowed: bool,
    pub batch: Batch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Surrounding {
    pub text: String,
    pub cursor: i32,
    pub anchor: i32,
    pub document_start: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChangeCause {
    #[default]
    InputMethod,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub target_id: u64,
    pub focus_id: u64,
    pub monitor_idx: usize,
    pub surrounding: Option<Surrounding>,
    pub cursor_rect: Option<(i32, i32, i32, i32)>,
    pub cause: ChangeCause,
    pub content: ContentType,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContentType {
    #[default]
    Multiline,
    Text,
    Digits,
    Number,
}

#[derive(Clone, Copy, Debug)]
pub struct FieldLayout {
    pub field_id: u64,
    pub revision: u64,
    pub font_size: u32,
    pub weight: u16,
    pub font_count: usize,
    pub cursor_x: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Focus {
    pub id: u64,
    pub monitor_idx: usize,
}
