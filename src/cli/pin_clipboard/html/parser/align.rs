//! 【贴图】【HTML对齐】维护 align 与 text-align 作用域并解析块级有效对齐

use super::super::model::Align;
use super::super::tags::tag_align;
use super::super::token::Tag;
use super::Parser;

impl Parser {
    /// 【贴图】【当前对齐】无参数，按居中容器与最近的对齐作用域返回当前对齐，缺省左对齐
    pub(super) fn current_align(&self) -> Align {
        if self.center > 0 {
            Align::Center
        } else {
            self.align_scope.last().copied().unwrap_or(Align::Left)
        }
    }

    /// 【贴图】【有效对齐】按 tag 返回居中容器、标签自身对齐、外层作用域依次决定的最终对齐
    pub(super) fn effective_align(&self, tag: &Tag) -> Align {
        if self.center > 0 {
            return Align::Center;
        }
        tag_align(tag)
            .or_else(|| self.align_scope.last().copied())
            .unwrap_or(Align::Left)
    }

    /// 【贴图】【对齐入栈】按 tag 的自有对齐建立作用域，并记录在最近一个标签帧上供关闭时弹出
    pub(super) fn enter_align_scope(&mut self, tag: &Tag) {
        let Some(align) = tag_align(tag) else {
            return;
        };
        self.align_scope.push(align);
        if let Some(frame) = self.frames.last_mut() {
            frame.align_pushed = true;
        }
    }
}
