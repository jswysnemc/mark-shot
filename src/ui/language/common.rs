//! 【界面语言】【通用文案】原版编辑菜单、复制反馈及窗口操作中文，不包含用户数据

/// 【界面语言】【通用查询】按 source 返回已核对的原版中文，未知键返回空值
pub(super) fn lookup(source: &str) -> Option<&'static str> {
    Some(match source {
        "Group %1" => "组合 %1",
        "Upload failed" => "上传失败",
        "Upload timed out" => "上传超时",
        "Upload canceled" => "已取消上传",
        "Image URL copied" => "已复制图片链接",
        "Undo" => "撤销",
        "Redo" => "重做",
        "Cut" => "剪切",
        "Copy" => "复制",
        "Paste" => "粘贴",
        "Delete" => "删除",
        "Select All" => "全选",
        "Copied" => "已复制",
        "Copy failed" => "复制失败",
        "Cancel" => "取消",
        "Edit" => "编辑",
        "Pin image" => "置顶图片",
        "Close" => "关闭",
        "Translate" => "翻译",
        "Always on Top" => "始终置顶",
        "Failed to save settings" => "保存设置失败",
        "Open with" => "打开方式",
        "Extensions" => "扩展",
        "No image desktop entries found" => "未找到图片桌面程序",
        "No extension commands configured.\nCreate %1" => "未配置扩展命令。\n请创建 %1",
        _ => return None,
    })
}
