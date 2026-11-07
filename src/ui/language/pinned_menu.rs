//! 【贴图菜单】【原版文案】对照ui/i18n.cpp和i18n_task_windows.cpp的完整操作名称

/// 【贴图菜单】【文案查询】按source返回原版中文菜单文案，未知键返回空值
pub(super) fn lookup(source: &str) -> Option<&'static str> {
    Some(match source {
        "Rotate Left" => "向左旋转",
        "Rotate Right" => "向右旋转",
        "Zoom In" => "放大",
        "Zoom Out" => "缩小",
        "Reset Size" => "重置大小",
        "Copy Selected Text" => "复制选中的文字",
        "Copy Image Text" => "复制图片文字",
        "Text Selection Copy" => "拖选复制文字",
        "Target Language" => "目标语言",
        "Show Original Text" => "显示原文",
        "Show Translated Text" => "显示译文",
        "Save As" => "另存为",
        _ => return None,
    })
}
