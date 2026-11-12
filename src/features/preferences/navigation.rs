//! 【设置窗口】【键盘导航】所有普通设置共用可见字段顺序与双向焦点移动
use super::{
    catalog::Page,
    window::{Control, Preferences},
};

#[cfg(test)]
mod tests;

pub(super) struct Location {
    page: Page,
    query: String,
    scroll: usize,
    focus: Option<usize>,
}

impl Preferences {
    /// 【设置导航】【进入子页】无参数，记录当前分类、搜索、滚动与焦点，再进入工具栏编辑，不改写配置
    pub(super) fn open_toolbar(&mut self) {
        let origin = Location {
            page: self.page,
            query: self.search.text().to_owned(),
            scroll: self.scroll,
            focus: self.focus,
        };
        self.activate(Control::Page(Page::Toolbar));
        self.toolbar_return = Some(origin);
        self.toolbar.prepare();
    }

    /// 【设置导航】【返回父页】无参数，恢复进入前的位置并取消子页手势，返回是否实际完成导航
    pub(super) fn back(&mut self) -> bool {
        let Some(origin) = self.toolbar_return.take() else {
            return false;
        };
        self.toolbar.cancel();
        self.toolbar.keyboard = None;
        self.pressed = None;
        self.scrollbar_drag = None;
        self.page = origin.page;
        self.search.set(&origin.query, &mut self.fonts, false);
        self.search_focus = false;
        self.focus = origin.focus;
        self.scroll = origin.scroll.min(super::layout::Layout::new(self).maximum);
        true
    }

    /// 【设置窗口】【焦点移动】按 reverse 选择上一个或下一个字段，自动显示目标行，无返回值
    pub(super) fn advance_focus(&mut self, reverse: bool) {
        let visible = self.visible();
        if visible.is_empty() {
            return;
        }
        let next = self
            .focus
            .and_then(|focus| visible.iter().position(|index| *index == focus))
            .map_or(if reverse { visible.len() - 1 } else { 0 }, |index| {
                if reverse {
                    (index + visible.len() - 1) % visible.len()
                } else {
                    (index + 1) % visible.len()
                }
            });
        self.focus = Some(visible[next]);
        self.reveal_row(visible[next]);
    }
}
