//! 【选区尺寸】【数值字段】待提交文字与实际尺寸分开保存，上下键延迟应用，滚轮立即请求调整

use crate::{
    features::selection_size::number::{self, Validation},
    types::text_input::Edit,
    ui::text_field::{LineEditState, composition::Update, validation::Rule},
};

pub struct NumberField {
    pub value: i32,
    pub maximum: i32,
    pub editing: bool,
    pub editor: LineEditState,
    pub id: u64,
    pub(crate) session: u64,
    pub(crate) scroll: super::scroll::Scroll,
    synced: String,
}

impl NumberField {
    /// 【选区尺寸】【字段构造】按 value 和 maximum 返回隐藏的数值字段，不分配输入法身份或读取字体
    pub fn new(value: i32, maximum: i32) -> Self {
        let maximum = maximum.max(1);
        Self {
            value: value.clamp(1, maximum),
            maximum,
            editing: false,
            editor: LineEditState::new(String::new()),
            id: 0,
            session: 0,
            scroll: super::scroll::Scroll::default(),
            synced: String::new(),
        }
    }

    /// 【选区尺寸】【数值同步】按 value 和 maximum 更新实际尺寸，用户改动过的待提交文字保持不变，无返回值
    pub fn sync(&mut self, value: i32, maximum: i32) {
        self.maximum = maximum.max(1);
        self.value = value.clamp(1, self.maximum);
        if self.editing && self.editor.text == self.synced {
            let text = self.value.to_string();
            if self.synced != text {
                self.replace(text, true);
            }
        }
    }

    /// 【选区尺寸】【开始编辑】用实际数值开始编辑并全选，已经编辑时只全选，无参数、无返回值
    pub fn begin(&mut self) {
        if self.editing {
            self.editor.select_all();
            return;
        }
        self.editing = true;
        self.replace(self.value.to_string(), true);
        // 1. 【选区尺寸】【编辑身份】本次编辑保持同一鼠标身份，程序改写只更新输入法身份
        self.session = self.id;
    }

    /// 【选区尺寸】【程序改写】按 text 更新输入并全选，synced 决定是否跟随尺寸回写，返回空值
    fn replace(&mut self, text: String, synced: bool) {
        // 1. 【选区尺寸】【输入重置】程序改写清理局部历史和候选，不继承旧输入法确认身份
        self.id = crate::features::text::input_method::focus::identity();
        self.editor = LineEditState::new(text.clone());
        self.editor.select_all();
        // 2. 【选区尺寸】【同步标记】滚轮输入允许接收实际限制结果，键盘待提交值继续由用户控制
        if synced {
            self.synced = text;
        }
    }

    /// 【选区尺寸】【结束编辑】隐藏当前输入并清理候选和同步标记，不回退已经应用的滚轮尺寸，无参数、无返回值
    pub fn cancel(&mut self) {
        self.editing = false;
        self.editor.cancel_preedit();
        self.synced.clear();
    }

    /// 【选区尺寸】【输入插入】验证完整 text 替换后才修改待提交文字，返回是否接受，不逐字符过滤粘贴
    pub fn insert(&mut self, text: &str) -> bool {
        if !self.editing || text.is_empty() {
            return false;
        }
        let selected = self
            .editor
            .selected_text()
            .map_or(0, |text| text.encode_utf16().count());
        let allowance = self.rule().maximum().saturating_sub(
            self.editor
                .text
                .encode_utf16()
                .count()
                .saturating_sub(selected),
        );
        let text = Rule::clipped(text, allowance);
        if text.is_empty() {
            return false;
        }
        self.edit(|editor| editor.insert_str(text))
    }

    /// 【选区尺寸】【原子字段动作】在副本上执行 operation，返回是否接受，删除和粘贴都不能留下 Qt 拒绝的正文
    pub fn edit(&mut self, operation: impl FnOnce(&mut LineEditState)) -> bool {
        if !self.editing {
            return false;
        }
        let mut proposed = self.editor.clone();
        operation(&mut proposed);
        if proposed.text.encode_utf16().count() > self.rule().maximum()
            || number::validate(&proposed.text, self.maximum) == Validation::Invalid
        {
            return false;
        }
        self.editor = proposed;
        true
    }

    /// 【选区尺寸】【提交数值】结束编辑并按普通十进制解析，返回限制后的尺寸，空文本和分组文本不应用，无参数
    pub fn commit(&mut self) -> Option<i32> {
        if !self.editing {
            return None;
        }
        let typed = self.editor.text.trim().parse::<i32>().ok();
        self.cancel();
        // 1. 【选区尺寸】【提交失焦】提交后按原版清除字段选择，Escape 取消保留原有隐藏输入状态
        self.editor.selection_anchor = None;
        let next = typed?.clamp(1, self.maximum);
        self.value = next;
        Some(next)
    }

    /// 【选区尺寸】【键盘微调】按 delta 修改待提交值并全选，无效文字以零开始，实际尺寸保持不变，无返回值
    pub fn arrow(&mut self, delta: i32) {
        if !self.editing {
            return;
        }
        let next = self
            .editor
            .text
            .trim()
            .parse::<i32>()
            .unwrap_or(0)
            .saturating_add(delta)
            .clamp(1, self.maximum);
        self.replace(next.to_string(), false);
    }

    /// 【选区尺寸】【滚轮微调】按 delta 返回立即应用的请求，编辑中的无效文字以实际值开始，并保留编辑状态
    pub fn wheel(&mut self, delta: i32) -> Option<i32> {
        let current = if self.editing {
            self.editor.text.trim().parse::<i32>().unwrap_or(self.value)
        } else {
            self.value
        };
        let next = current.saturating_add(delta).clamp(1, self.maximum);
        if self.editing {
            self.replace(next.to_string(), true);
            if next != self.value {
                self.value = next;
                return Some(next);
            }
            None
        } else {
            Some(next)
        }
    }

    /// 【选区尺寸】【验证规则】返回当前上限对应的原生整数字段规则，无参数，不声明数字输入法提示
    pub fn rule(&self) -> Rule {
        Rule::SelectionSize {
            maximum: self.maximum,
        }
    }

    /// 【选区尺寸】【原生编辑】按 event 原子处理确认与候选，返回实际变化，不把待提交值直接写入选区
    pub fn input_method(&mut self, event: Edit) -> Update {
        if !self.editing {
            return Update::default();
        }
        let rule = self.rule();
        self.editor.input_method_update(event, rule)
    }
}
