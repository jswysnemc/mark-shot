//! 【编辑启动】【工具选择】保留原版普通、全屏和文件工具的逐级覆盖关系

use super::Launch;
use crate::tools::Tool;

/// 【编辑启动】【初始工具】按 launch 返回全屏或图片编辑工具；显式配置优先，图片默认选择标注，截图默认选择范围
pub(super) fn initial(launch: &Launch) -> Tool {
    let file = if launch.image.is_some() {
        launch.file_default_tool
    } else {
        None
    };
    file.or(launch.fullscreen_default_tool)
        .or(launch.default_tool)
        .unwrap_or(if launch.image.is_some() {
            Tool::Pick
        } else {
            Tool::Selection
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 【编辑启动回归】【原版继承】普通工具、全屏工具按原版顺序为文件提供默认值，无参数、无返回值
    #[test]
    fn file_tool_inherits_normal_then_fullscreen_defaults() {
        let mut launch = Launch {
            image: Some(PathBuf::from("picture.png")),
            ..Launch::default()
        };
        assert_eq!(initial(&launch), Tool::Pick);
        launch.default_tool = Some(Tool::Pen);
        assert_eq!(initial(&launch), Tool::Pen);
        launch.fullscreen_default_tool = Some(Tool::Ruler);
        assert_eq!(initial(&launch), Tool::Ruler);
        launch.file_default_tool = Some(Tool::Rectangle);
        assert_eq!(initial(&launch), Tool::Rectangle);
    }

    /// 【编辑启动回归】【文件隔离】文件工具不能覆盖屏幕全屏工具或普通截图工具，无参数、无返回值
    #[test]
    fn file_override_does_not_change_screen_launch() {
        let mut launch = Launch {
            file_default_tool: Some(Tool::Rectangle),
            default_tool: Some(Tool::Pen),
            fullscreen_default_tool: Some(Tool::Ruler),
            fullscreen: true,
            ..Launch::default()
        };
        assert_eq!(initial(&launch), Tool::Ruler);
        launch.fullscreen_default_tool = None;
        assert_eq!(initial(&launch), Tool::Pen);
    }
}
