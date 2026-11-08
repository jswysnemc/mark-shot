//! 【设置窗口】【业务同步】只同步用户实际修改的会话参数，不重建截图、不改动标注历史

use crate::{config::Config, editor::EditorState};

/// 【设置窗口】【编辑器恢复】比较 before、after 并将变化应用到 state，dirty 返回重绘；启动捕获参数留到下次截图
pub fn apply(state: &mut EditorState, before: &Config, after: &Config, dirty: &mut u32) {
    if before.toolbar_layout != after.toolbar_layout {
        crate::app::configure_toolbar(state, &after.toolbar_layout);
    }
    if before.general.to != after.general.to {
        state.to = after.general.to;
    }
    if before.startup.selection_loupe != after.startup.selection_loupe {
        state.magnifier.hidden = after.startup.selection_loupe == Some(false);
    }
    if before.startup.normal_tool != after.startup.normal_tool
        || before.startup.default_tool != after.startup.default_tool
    {
        state.after_selection_tool = after
            .startup
            .normal_tool
            .as_deref()
            .or(after.startup.default_tool.as_deref())
            .and_then(|name| crate::tools::names::parse(name).ok())
            .unwrap_or(crate::tools::Tool::Selection);
    }
    crate::editor::dirty::mark_all_dirty(dirty, state.placements.len());
}
