//! 【设置窗口】【键盘导航】所有普通设置共用可见字段顺序与双向焦点移动
use super::window::Preferences;
impl Preferences {
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
