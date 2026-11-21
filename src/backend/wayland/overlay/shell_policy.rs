//! 【截图输入法】【窗口策略】KWin 候选窗口需要普通截图窗口，避免覆盖层遮挡

/// 【截图输入法】【桌面识别】按 desktop 的标准冒号分隔值判断是否为 KDE，返回识别结果
fn kde(desktop: &str) -> bool {
    desktop
        .split(':')
        .any(|name| name.eq_ignore_ascii_case("KDE") || name.eq_ignore_ascii_case("PLASMA"))
}

/// 【截图输入法】【角色选择】按 requested 检查当前桌面，返回是否使用截图浮层；KDE 的输入法候选必须高于编辑窗口
pub(super) fn layer_allowed(requested: bool) -> bool {
    if requested && kde(&std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default()) {
        log::info!("【截图输入法】【窗口选择】KDE 截图使用普通窗口，避免浮层遮挡输入法候选框");
        return false;
    }
    requested
}

#[cfg(test)]
mod tests {
    /// 【截图输入法测试】【桌面判断】无参数，标准桌面列表精确识别 KDE，不误判其他合成器，无返回值
    #[test]
    fn matches_desktop_tokens() {
        assert!(super::kde("KDE"));
        assert!(super::kde("ubuntu:plasma"));
        assert!(!super::kde("niri"));
        assert!(!super::kde("GNOME"));
        assert!(!super::kde("not-KDE"));
    }
}
